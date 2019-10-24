"""
    To gather information.

    Build:
        cargo build --all

    Run node with:
        ./target/debug/near --verbose network_metrics run 2>| tee watch.log

    Gather data
        python3 script-aggregate-network-output.py < watch.log
"""
# Aggregate some metrics.
import re
from pprint import pprint
from itertools import count

try:
    from tqdm import tqdm
except:
    tqdm = lambda x : x

pattern = re.compile(r"^\[([A-Z0-9-:]*) .*\] Type=(\w*) Size=(\d*) From=Some\(([\w:]*)\)")

class SuperDict:
    def __init__(self):
        self._count = {}
        self._size = {}

    def add(self, key, value):
        value = int(value)
        self._count[key] = self._count.get(key, 0) + 1
        self._size[key] = self._size.get(key, 0) + value

    def show(self):
        print("\nCount")
        pprint(self._count)
        print("Size")
        pprint(self._size)


mt = SuperDict()
pf = SuperDict()

for _ in tqdm(count()):
    try:
        line = input()
    except:
        break

    answer = pattern.match(line)

    if answer is None:
        continue

    time, msg_type, size, peer_from = pattern.match(line).groups()
    mt.add(msg_type, size)
    pf.add(peer_from, size)

mt.show()
pf.show()
