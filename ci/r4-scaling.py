#!/usr/bin/env python3
"""Apply the unchanged R4 scaling/time bounds to a recorded dense-checker run."""
import json,sys
from pathlib import Path
rows=[json.loads(line) for line in Path(sys.argv[1]).read_text().splitlines()]
assert len(rows)==2,'expected 10k/100k measurements'
small,large=sorted(rows,key=lambda row:row['events'])
assert small['events']==10000 and large['events']==100000,'wrong event counts'
assert large['findings']==10*small['findings'],'wrong violation density'
ratio=large['median_ns']/small['median_ns']
print(f"R4 medians: {small['median_ns']} ns -> {large['median_ns']} ns; ratio {ratio:.6f}; limit 12",flush=True)
assert large['median_ns']<=500_000_000,'R4 absolute time exceeds 0.5 seconds'
assert large['median_ns']<=12*small['median_ns'],'R4 scaling criterion missed (threshold unchanged)'
print('R4 scaling: PASS')
