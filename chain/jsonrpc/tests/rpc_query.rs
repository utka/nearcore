#![recursion_limit = "256"]
use std::convert::TryFrom;

use actix::{Actor, System};
use futures::{future, FutureExt};

use near_crypto::{KeyType, PublicKey, Signature};
use near_jsonrpc::client::new_client;
use near_jsonrpc_client::ChunkId;
use near_network::test_utils::WaitOrTimeout;
use near_primitives::account::{AccessKey, AccessKeyPermission};
use near_primitives::hash::CryptoHash;
use near_primitives::rpc::{RpcGenesisRecordsRequest, RpcPagination, RpcQueryRequest};
use near_primitives::test_utils::init_test_logger;
use near_primitives::types::{BlockId, ShardId};
use near_primitives::views::{Finality, QueryRequest, QueryResponseKind};

mod test_utils;

macro_rules! test_with_client {
    ($client:ident, $block:expr) => {
        init_test_logger();

        System::run(|| {
            let (_view_client_addr, addr) = test_utils::start_all(false);

            let mut $client = new_client(&format!("http://{}", addr));

            actix::spawn(async move {
                $block.await;
                System::current().stop();
            });
        })
        .unwrap();
    };
}

/// Retrieve blocks via json rpc
#[test]
fn test_block() {
    test_with_client!(client, async move {
        let block = client.block(BlockId::Height(0)).await.unwrap();
        assert_eq!(block.author, "test1");
        assert_eq!(block.header.height, 0);
        assert_eq!(block.header.epoch_id.0.as_ref(), &[0; 32]);
        assert_eq!(block.header.hash.0.as_ref().len(), 32);
        assert_eq!(block.header.prev_hash.0.as_ref(), &[0; 32]);
        assert_eq!(
            block.header.prev_state_root,
            CryptoHash::try_from("7tkzFg8RHBmMw1ncRJZCCZAizgq4rwCftTKYLce8RU8t").unwrap()
        );
        assert!(block.header.timestamp > 0);
        assert_eq!(block.header.validator_proposals.len(), 0);
    });
}

/// Retrieve blocks via json rpc
#[test]
fn test_block_by_hash() {
    test_with_client!(client, async move {
        let block = client.block(BlockId::Height(0)).await.unwrap();
        let same_block = client.block(BlockId::Hash(block.header.hash)).await.unwrap();
        assert_eq!(block.header.height, 0);
        assert_eq!(same_block.header.height, 0);
    });
}

/// Retrieve chunk via json rpc
#[test]
fn test_chunk_by_hash() {
    test_with_client!(client, async move {
        let chunk = client
            .chunk(ChunkId::BlockShardId(BlockId::Height(0), ShardId::from(0u64)))
            .await
            .unwrap();
        assert_eq!(chunk.author, "test2");
        assert_eq!(chunk.header.balance_burnt, 0);
        assert_eq!(chunk.header.chunk_hash.as_ref().len(), 32);
        assert_eq!(chunk.header.encoded_length, 8);
        assert_eq!(chunk.header.encoded_merkle_root.as_ref().len(), 32);
        assert_eq!(chunk.header.gas_limit, 1000000);
        assert_eq!(chunk.header.gas_used, 0);
        assert_eq!(chunk.header.height_created, 0);
        assert_eq!(chunk.header.height_included, 0);
        assert_eq!(chunk.header.outgoing_receipts_root.as_ref().len(), 32);
        assert_eq!(chunk.header.prev_block_hash.as_ref().len(), 32);
        assert_eq!(chunk.header.prev_state_root.as_ref().len(), 32);
        assert_eq!(chunk.header.rent_paid, 0);
        assert_eq!(chunk.header.shard_id, 0);
        assert!(if let Signature::ED25519(_) = chunk.header.signature { true } else { false });
        assert_eq!(chunk.header.tx_root.as_ref(), &[0; 32]);
        assert_eq!(chunk.header.validator_proposals, vec![]);
        assert_eq!(chunk.header.validator_reward, 0);
        let same_chunk = client.chunk(ChunkId::Hash(chunk.header.chunk_hash)).await.unwrap();
        assert_eq!(chunk.header.chunk_hash, same_chunk.header.chunk_hash);
    });
}

/// Retrieve chunk via json rpc
#[test]
fn test_chunk_invalid_shard_id() {
    test_with_client!(client, async move {
        let chunk = client.chunk(ChunkId::BlockShardId(BlockId::Height(0), 100)).await;
        match chunk {
            Ok(_) => panic!("should result in an error"),
            Err(e) => {
                let s = serde_json::to_string(&e.data.unwrap()).unwrap();
                assert!(s.starts_with("\"Shard id 100 does not exist"));
            }
        }
    });
}

/// Connect to json rpc and query account info with soft-deprecated query API.
#[test]
fn test_query_by_path_account() {
    test_with_client!(client, async move {
        let status = client.status().await.unwrap();
        let block_hash = status.sync_info.latest_block_hash;
        let query_response =
            client.query_by_path("account/test".to_string(), "".to_string()).await.unwrap();
        assert_eq!(query_response.block_height, 0);
        assert_eq!(query_response.block_hash, block_hash);
        let account_info = if let QueryResponseKind::ViewAccount(account) = query_response.kind {
            account
        } else {
            panic!("queried account, but received something else: {:?}", query_response.kind);
        };
        assert_eq!(account_info.amount, 0);
        assert_eq!(account_info.code_hash.as_ref(), &[0; 32]);
        assert_eq!(account_info.locked, 0);
        assert_eq!(account_info.storage_paid_at, 0);
        assert_eq!(account_info.storage_usage, 0);
    });
}

/// Connect to json rpc and query account info.
#[test]
fn test_query_account() {
    test_with_client!(client, async move {
        let status = client.status().await.unwrap();
        let block_hash = status.sync_info.latest_block_hash;
        let query_response_1 = client
            .query(RpcQueryRequest {
                block_id: None,
                request: QueryRequest::ViewAccount { account_id: "test".to_string() },
                finality: Finality::None,
            })
            .await
            .unwrap();
        let query_response_2 = client
            .query(RpcQueryRequest {
                block_id: Some(BlockId::Height(0)),
                request: QueryRequest::ViewAccount { account_id: "test".to_string() },
                finality: Finality::None,
            })
            .await
            .unwrap();
        let query_response_3 = client
            .query(RpcQueryRequest {
                block_id: Some(BlockId::Hash(block_hash)),
                request: QueryRequest::ViewAccount { account_id: "test".to_string() },
                finality: Finality::None,
            })
            .await
            .unwrap();
        let query_response_4 = client
            .query(RpcQueryRequest {
                block_id: Some(BlockId::Hash(block_hash)),
                request: QueryRequest::ViewAccount { account_id: "test".to_string() },
                finality: Finality::DoomSlug,
            })
            .await
            .unwrap();
        let query_response_5 = client
            .query(RpcQueryRequest {
                block_id: Some(BlockId::Hash(block_hash)),
                request: QueryRequest::ViewAccount { account_id: "test".to_string() },
                finality: Finality::NFG,
            })
            .await
            .unwrap();
        for query_response in [
            query_response_1,
            query_response_2,
            query_response_3,
            query_response_4,
            query_response_5,
        ]
        .into_iter()
        {
            assert_eq!(query_response.block_height, 0);
            assert_eq!(query_response.block_hash, block_hash);
            let account_info = if let QueryResponseKind::ViewAccount(ref account) =
                query_response.kind
            {
                account
            } else {
                panic!("queried account, but received something else: {:?}", query_response.kind);
            };
            assert_eq!(account_info.amount, 0);
            assert_eq!(account_info.code_hash.as_ref(), &[0; 32]);
            assert_eq!(account_info.locked, 0);
            assert_eq!(account_info.storage_paid_at, 0);
            assert_eq!(account_info.storage_usage, 0);
        }
    });
}

/// Connect to json rpc and query account info with soft-deprecated query API.
#[test]
fn test_query_by_path_access_keys() {
    test_with_client!(client, async move {
        let query_response =
            client.query_by_path("access_key/test".to_string(), "".to_string()).await.unwrap();
        assert_eq!(query_response.block_height, 0);
        let access_keys = if let QueryResponseKind::AccessKeyList(access_keys) = query_response.kind
        {
            access_keys
        } else {
            panic!("queried access keys, but received something else: {:?}", query_response.kind);
        };
        assert_eq!(access_keys.keys.len(), 1);
        assert_eq!(access_keys.keys[0].access_key, AccessKey::full_access().into());
        assert_eq!(access_keys.keys[0].public_key, PublicKey::empty(KeyType::ED25519));
    });
}

/// Connect to json rpc and query account info.
#[test]
fn test_query_access_keys() {
    test_with_client!(client, async move {
        let query_response = client
            .query(RpcQueryRequest {
                block_id: None,
                request: QueryRequest::ViewAccessKeyList { account_id: "test".to_string() },
                finality: Finality::None,
            })
            .await
            .unwrap();
        assert_eq!(query_response.block_height, 0);
        let access_keys = if let QueryResponseKind::AccessKeyList(access_keys) = query_response.kind
        {
            access_keys
        } else {
            panic!("queried access keys, but received something else: {:?}", query_response.kind);
        };
        assert_eq!(access_keys.keys.len(), 1);
        assert_eq!(access_keys.keys[0].access_key, AccessKey::full_access().into());
        assert_eq!(access_keys.keys[0].public_key, PublicKey::empty(KeyType::ED25519));
    });
}

/// Connect to json rpc and query account info with soft-deprecated query API.
#[test]
fn test_query_by_path_access_key() {
    test_with_client!(client, async move {
        let query_response = client
            .query_by_path(
                "access_key/test/ed25519:23vYngy8iL7q94jby3gszBnZ9JptpMf5Hgf7KVVa2yQ2".to_string(),
                "".to_string(),
            )
            .await
            .unwrap();
        assert_eq!(query_response.block_height, 0);
        let access_key = if let QueryResponseKind::AccessKey(access_keys) = query_response.kind {
            access_keys
        } else {
            panic!("queried access keys, but received something else: {:?}", query_response.kind);
        };
        assert_eq!(access_key.nonce, 0);
        assert_eq!(access_key.permission, AccessKeyPermission::FullAccess.into());
    });
}

/// Connect to json rpc and query account info.
#[test]
fn test_query_access_key() {
    test_with_client!(client, async move {
        let query_response = client
            .query(RpcQueryRequest {
                block_id: None,
                request: QueryRequest::ViewAccessKey {
                    account_id: "test".to_string(),
                    public_key: PublicKey::try_from(
                        "ed25519:23vYngy8iL7q94jby3gszBnZ9JptpMf5Hgf7KVVa2yQ2",
                    )
                    .unwrap(),
                },
                finality: Finality::None,
            })
            .await
            .unwrap();
        assert_eq!(query_response.block_height, 0);
        let access_key = if let QueryResponseKind::AccessKey(access_keys) = query_response.kind {
            access_keys
        } else {
            panic!("queried access keys, but received something else: {:?}", query_response.kind);
        };
        assert_eq!(access_key.nonce, 0);
        assert_eq!(access_key.permission, AccessKeyPermission::FullAccess.into());
    });
}

/// Connect to json rpc and query state.
#[test]
fn test_query_state() {
    test_with_client!(client, async move {
        let query_response = client
            .query(RpcQueryRequest {
                block_id: None,
                request: QueryRequest::ViewState {
                    account_id: "test".to_string(),
                    prefix: vec![].into(),
                },
                finality: Finality::None,
            })
            .await
            .unwrap();
        assert_eq!(query_response.block_height, 0);
        let state = if let QueryResponseKind::ViewState(state) = query_response.kind {
            state
        } else {
            panic!("queried state, but received something else: {:?}", query_response.kind);
        };
        assert_eq!(state.values.len(), 0);
    });
}

/// Connect to json rpc and call function
#[test]
fn test_query_call_function() {
    test_with_client!(client, async move {
        let query_response = client
            .query(RpcQueryRequest {
                block_id: None,
                request: QueryRequest::CallFunction {
                    account_id: "test".to_string(),
                    method_name: "method".to_string(),
                    args: vec![].into(),
                },
                finality: Finality::None,
            })
            .await
            .unwrap();
        assert_eq!(query_response.block_height, 0);
        let call_result = if let QueryResponseKind::CallResult(call_result) = query_response.kind {
            call_result
        } else {
            panic!(
                "expected a call function result, but received something else: {:?}",
                query_response.kind
            );
        };
        assert_eq!(call_result.result.len(), 0);
        assert_eq!(call_result.logs.len(), 0);
    });
}

/// Retrieve client status via JSON RPC.
#[test]
fn test_status() {
    test_with_client!(client, async move {
        let status = client.status().await.unwrap();
        assert_eq!(status.chain_id, "unittest");
        assert_eq!(status.sync_info.latest_block_height, 0);
        assert_eq!(status.sync_info.syncing, false);
    });
}

/// Retrieve client status failed.
#[test]
fn test_status_fail() {
    init_test_logger();

    System::run(|| {
        let (_, addr) = test_utils::start_all(false);

        let mut client = new_client(&format!("http://{}", addr));
        WaitOrTimeout::new(
            Box::new(move |_| {
                actix::spawn(client.health().then(|res| {
                    if res.is_err() {
                        System::current().stop();
                    }
                    future::ready(())
                }));
            }),
            100,
            10000,
        )
        .start();
    })
    .unwrap();
}

/// Check health fails when node is absent.
#[test]
fn test_health_fail() {
    init_test_logger();

    System::run(|| {
        let mut client = new_client(&"http://127.0.0.1:12322/health");
        actix::spawn(client.health().then(|res| {
            assert!(res.is_err());
            System::current().stop();
            future::ready(())
        }));
    })
    .unwrap();
}

/// Health fails when node doesn't produce block for period of time.
#[test]
fn test_health_fail_no_blocks() {
    init_test_logger();

    System::run(|| {
        let (_, addr) = test_utils::start_all(false);

        let mut client = new_client(&format!("http://{}", addr));
        WaitOrTimeout::new(
            Box::new(move |_| {
                actix::spawn(client.health().then(|res| {
                    if res.is_err() {
                        System::current().stop();
                    }
                    future::ready(())
                }));
            }),
            300,
            10000,
        )
        .start();
    })
    .unwrap();
}

/// Retrieve client health.
#[test]
fn test_health_ok() {
    test_with_client!(client, async move {
        let health = client.health().await;
        assert!(health.is_ok());
    });
}

/// Retrieve genesis config via JSON RPC.
/// WARNING: Be mindful about changing genesis structure as it is part of the public protocol!
#[test]
fn test_genesis_config() {
    test_with_client!(client, async move {
        let genesis_config = client.EXPERIMENTAL_genesis_config().await.unwrap();
        assert_eq!(
            genesis_config,
            serde_json::json!({
                "config_version": 1,
                "protocol_version": 4,
                "genesis_time": "2019-06-04T06:13:25Z",
                "chain_id": "testnet",
                "num_block_producer_seats": 50,
                "num_block_producer_seats_per_shard": [7, 7, 6, 6, 6, 6, 6, 6],
                "avg_hidden_validator_seats_per_shard": [0, 0, 0, 0, 0, 0, 0, 0],
                "dynamic_resharding": false,
                "epoch_length": 600,
                "gas_limit": 1000000000000000u64,
                "min_gas_price": "5000",
                "block_producer_kickout_threshold": 80,
                "chunk_producer_kickout_threshold": 60,
                "gas_price_adjustment_rate": 1,
                "runtime_config": {
                    "storage_cost_byte_per_block": "5000000",
                    "poke_threshold": 86400,
                    "transaction_costs": {
                        "action_receipt_creation_config": {
                            "send_sir": 924119500000u64,
                            "send_not_sir": 924119500000u64,
                            "execution": 924119500000u64
                        },
                        "data_receipt_creation_config": {
                            "base_cost": {
                                "send_sir": 539890689500u64,
                                "send_not_sir": 539890689500u64,
                                "execution": 539890689500u64
                            },
                            "cost_per_byte": {
                                "send_sir": 14234654,
                                "send_not_sir": 14234654,
                                "execution": 14234654
                            }
                        },
                        "action_creation_config": {
                            "create_account_cost": {
                                "send_sir": 0,
                                "send_not_sir": 0,
                                "execution": 0
                            },
                            "deploy_contract_cost": {
                                "send_sir": 513359000000u64,
                                "send_not_sir": 513359000000u64,
                                "execution": 513359000000u64
                            },
                            "deploy_contract_cost_per_byte": {
                                "send_sir": 27106233,
                                "send_not_sir": 27106233,
                                "execution": 27106233
                            },
                            "function_call_cost": {
                                "send_sir": 1367372500000u64,
                                "send_not_sir": 1367372500000u64,
                                "execution": 1367372500000u64
                            },
                            "function_call_cost_per_byte": {
                                "send_sir": 2354953,
                                "send_not_sir": 2354953,
                                "execution": 2354953
                            },
                            "transfer_cost": {
                                "send_sir": 13025000000u64,
                                "send_not_sir": 13025000000u64,
                                "execution": 13025000000u64
                            },
                            "stake_cost": {
                                "send_sir": 0,
                                "send_not_sir": 0,
                                "execution": 0
                            },
                            "add_key_cost": {
                                "full_access_cost": {
                                    "send_sir": 0,
                                    "send_not_sir": 0,
                                    "execution": 0
                                },
                                "function_call_cost": {
                                    "send_sir": 0,
                                    "send_not_sir": 0,
                                    "execution": 0
                                },
                                "function_call_cost_per_byte": {
                                    "send_sir": 37538150u64,
                                    "send_not_sir": 37538150u64,
                                    "execution": 37538150u64
                                }
                            },
                            "delete_key_cost": {
                                "send_sir": 0,
                                "send_not_sir": 0,
                                "execution": 0
                            },
                            "delete_account_cost": {
                                "send_sir": 454830000000u64,
                                "send_not_sir": 454830000000u64,
                                "execution": 454830000000u64
                            }
                        },
                        "storage_usage_config": {
                            "account_cost": 100,
                            "data_record_cost": 40,
                            "key_cost_per_byte": 1,
                            "value_cost_per_byte": 1,
                            "code_cost_per_byte": 1
                        },
                        "burnt_gas_reward": {
                            "numerator": 3,
                            "denominator": 10
                        }
                    },
                    "wasm_config": {
                        "ext_costs": {
                            "base": 126224222,
                            "read_memory_base": 1629369577,
                            "read_memory_byte": 123816,
                            "write_memory_base": 76445225,
                            "write_memory_byte": 809907,
                            "read_register_base": 639340699,
                            "read_register_byte": 63637,
                            "write_register_base": 0,
                            "write_register_byte": 0,
                            "utf8_decoding_base": 0,
                            "utf8_decoding_byte": 591904,
                            "utf16_decoding_base": 0,
                            "utf16_decoding_byte": 9095538,
                            "sha256_base": 710092630,
                            "sha256_byte": 5536829,
                            "keccak256_base": 710092630,
                            "keccak256_byte": 5536829,
                            "keccak512_base": 1420185260,
                            "keccak512_byte": 11073658,
                            "log_base": 0,
                            "log_byte": 0,
                            "storage_write_base": 21058769282u64,
                            "storage_write_key_byte": 23447086,
                            "storage_write_value_byte": 9437547,
                            "storage_write_evicted_byte": 0,
                            "storage_read_base": 19352220621u64,
                            "storage_read_key_byte": 4792496,
                            "storage_read_value_byte": 139743,
                            "storage_remove_base": 109578968621u64,
                            "storage_remove_key_byte": 9512022,
                            "storage_remove_ret_value_byte": 0,
                            "storage_has_key_base": 20019912030u64,
                            "storage_has_key_byte": 4647597,
                            "storage_iter_create_prefix_base": 28443562030u64,
                            "storage_iter_create_prefix_byte": 442354,
                            "storage_iter_create_range_base": 25804628282u64,
                            "storage_iter_create_from_byte": 429608,
                            "storage_iter_create_to_byte": 1302886,
                            "storage_iter_next_base": 24213271567u64,
                            "storage_iter_next_key_byte": 0,
                            "storage_iter_next_value_byte": 1343211668,
                            "touching_trie_node": 1,
                            "promise_and_base": 0,
                            "promise_and_per_promise": 672136,
                            "promise_return": 34854215,
                        },
                        "grow_mem_cost": 1,
                        "regular_op_cost": 3856371,
                        "limit_config": {
                            "max_gas_burnt": 200000000000000u64,
                            "max_gas_burnt_view": 200000000000000u64,
                            "max_stack_height": 16384,
                            "initial_memory_pages": 1024,
                            "max_memory_pages": 2048,
                            "registers_memory_limit": 1073741824,
                            "max_register_size": 104857600,
                            "max_number_registers": 100,
                            "max_number_logs": 100,
                            "max_total_log_length": 16384,
                            "max_total_prepaid_gas": 10000000000000000u64,
                            "max_actions_per_receipt": 100,
                            "max_number_bytes_method_names": 2000,
                            "max_length_method_name": 256,
                            "max_arguments_length": 4194304,
                            "max_length_returned_data": 4194304,
                            "max_contract_size": 4194304,
                            "max_length_storage_key": 4194304,
                            "max_length_storage_value": 4194304,
                            "max_promises_per_function_call_action": 1024,
                            "max_number_input_data_dependencies": 128
                        },
                    },
                    "account_length_baseline_cost_per_block": "207909813343189798558"
                },
                "validators": [
                    {
                        "account_id": "far",
                        "public_key": "ed25519:7rNEmDbkn8grQREdTt3PWhR1phNtsqJdgfV26XdR35QL",
                        "amount": "200000000000000000000000000000000000"
                    }
                ],
                "fishermen_threshold": "10000000000000000000",
                "transaction_validity_period": 1000,
                "developer_reward_percentage": 30,
                "protocol_reward_percentage": 10,
                "max_inflation_rate": 5,
                "total_supply": "1638390651983309491821349612944000000",
                "num_blocks_per_year": 31536000,
                "protocol_treasury_account": "near"
            })
        );
    });
}

/// Retrieve genesis records via JSON RPC.
#[test]
fn test_genesis_records() {
    test_with_client!(client, async move {
        let genesis_records = client
            .EXPERIMENTAL_genesis_records(RpcGenesisRecordsRequest {
                pagination: Default::default(),
            })
            .await
            .unwrap();
        assert_eq!(genesis_records.records.len(), 100);
        let first100_records = genesis_records.records.to_vec();

        let second_genesis_record = client
            .EXPERIMENTAL_genesis_records(RpcGenesisRecordsRequest {
                pagination: RpcPagination { offset: 1, limit: 1 },
            })
            .await
            .unwrap();
        assert_eq!(second_genesis_record.records.len(), 1);

        assert_eq!(
            serde_json::to_value(&first100_records[1]).unwrap(),
            serde_json::to_value(&second_genesis_record.records[0]).unwrap()
        );
    });
}

/// Check invalid arguments to genesis records via JSON RPC.
#[test]
fn test_invalid_genesis_records_arguments() {
    test_with_client!(client, async move {
        let genesis_records_response = client
            .EXPERIMENTAL_genesis_records(RpcGenesisRecordsRequest {
                pagination: RpcPagination { offset: 1, limit: 101 },
            })
            .await;
        let validation_error = genesis_records_response.err().unwrap();
        assert_eq!(validation_error.code, -32_602);
        assert_eq!(validation_error.message, "Invalid params");
        assert_eq!(
            validation_error.data.unwrap(),
            serde_json::json!({
                "pagination": {
                    "limit": [
                        {
                            "code": "range",
                            "message": null,
                            "params": {
                                "max": 100.0,
                                "value": 101,
                                "min": 1.0,
                            }
                        }
                    ]
                }
            })
        );
    });
}

/// Retrieve gas price
#[test]
fn test_gas_price_by_height() {
    test_with_client!(client, async move {
        let gas_price = client.gas_price(Some(BlockId::Height(0))).await.unwrap();
        assert!(gas_price.gas_price > 0);
    });
}

/// Retrieve gas price
#[test]
fn test_gas_price_by_hash() {
    test_with_client!(client, async move {
        let block = client.block(BlockId::Height(0)).await.unwrap();
        let gas_price = client.gas_price(Some(BlockId::Hash(block.header.hash))).await.unwrap();
        assert!(gas_price.gas_price > 0);
    });
}

/// Retrieve gas price
#[test]
fn test_gas_price() {
    test_with_client!(client, async move {
        let gas_price = client.gas_price(None).await.unwrap();
        assert!(gas_price.gas_price > 0);
    });
}
