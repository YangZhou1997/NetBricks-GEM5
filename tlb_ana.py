import os
import sys
from collections import defaultdict

path = './target/x86_64-unknown-linux-musl/release/'
readelf = 'readelf -SW {task} | perl -pe "s/(0x)?[0-9a-f]{{5,}}/hex $&/ge" | grep A'

ro = [0 for i in range(6)]
rw = [0 for i in range(6)]
re = [0 for i in range(6)]
maxmem = list(map(lambda x: int(x * 1024 * 1024), [17.19921875, 51.14453125, 43.875, 13.80078125, 68.328125, 360.54296875]))

pgszs = [[2 * 1024 * 1024], [128 * 1024, 2 * 1024 * 1024, 64 * 1024 * 1024], [2 * 1024 * 1024, 32 * 1024 * 1024, 128 * 1024 * 1024]]

def get_group(ori_name):
    switcher = {
        **dict.fromkeys(["A", "AMS"], "RO"), 
        **dict.fromkeys(["WAT", "WA"], "RW"), 
        **dict.fromkeys(["AX"], "RE"), 
    }
    return switcher.get(ori_name, "Invalid name %s" % (ori_name,))

def get_tlb_num(mem_ls, pagesz):
    cnt = 0
    for mem in mem_ls:
        # print(pagesz[::-1])
        for p in pagesz[::-1]:
            # print(p, mem // p)
            cnt += mem // p
            mem -= (mem // p) * p
        if mem != 0:
            cnt += 1        
    return cnt

if __name__ == '__main__':
    tasks = ["acl-fw-ipsec", "dpi-ipsec", "nat-tcp-v4-ipsec", "maglev-ipsec", "lpm-ipsec", "monitoring-ipsec"]
    # tasks = ["nat-tcp-v4-ipsec"]

    cnt = 0
    for _task in tasks:
        results = os.popen(readelf.format(task=(path + _task))).read()
        resArray = results.split("\n")[1:-2]
        size_array = defaultdict(list)

        resArray = list(map(lambda x: x[7:].split(), resArray))
        for i in range(15):
            section = resArray[i]
            size = section[4]
            group = get_group(section[6])
            size_array[group].append(int(size))
        ro[cnt] = sum(size_array["RO"])
        rw[cnt] = sum(size_array["RW"])
        re[cnt] = sum(size_array["RE"])
        cnt += 1
    # print(ro, rw, re, maxmem)
    for i in range(6):
        heapstack = maxmem[i] - ro[i] - rw[i] - re[i]
        sys.stdout.write("%.02lf & %.02lf & %.02lf & %.02lf & %.02lf & " % (ro[i] / (1024 * 1024.0), rw[i] / (1024 * 1024.0), re[i] / (1024 * 1024.0), heapstack / (1024 * 1024.0), maxmem[i] / (1024 * 1024.0)))
        for pgset in pgszs:
            mem_ls = [ro[i], rw[i], re[i], heapstack]
            sys.stdout.write("%d & " % (get_tlb_num(mem_ls, pgset)))
        sys.stdout.write("\n")
