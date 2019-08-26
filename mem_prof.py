import os
import time
from termcolor import colored
import datetime
import threading
from threading import Event, Thread

# 5 pages from the monitoring to dpdk 
# cgroup seems including dpdk memory usage
# DPDK_MEM = (5 * 2 * 1024 * 1024)

# seems you cannot pass 1.2M to cgroup. 
# you should first do "export TMPDIR=/tmp"
CmdLimitMemBg = {
'start': 'export TMPDIR=/tmp && ./limitmem {mem}K bash ./run_gem5.sh {task}',
'kill': 'sudo pkill limitmem && sudo pkill head && sudo pkill {task}'
}

CmdGetCgroupID = {
'start': 'ps -ef | grep limitmem',
}

CmdGetCgroupMemUsage = {
'start': 'cgget -g {cgroup_name} | grep memory.memsw.usage_in_bytes',
'start_all': 'cgget -g {cgroup_name}',
}

CmdGetCgroupMaxMemUsage = {
'start': 'cgget -g {cgroup_name} | grep memory.memsw.max_usage_in_bytes',
}


def get_cgroup_name():
    start_string = '/run/user/20001/limitmem_'
    end_string = '_cgroup_closer'

    grep_results = os.popen(CmdGetCgroupID['start']).read()
    # print grep_results

    # find the latest cgroup name
    start_index = grep_results.rfind(start_string) + len(start_string) 
    # this task executes error. 
    if start_index == -1:
    	return "Err" 
    end_index = grep_results.find(end_string, start_index)
    if end_index == -1:
    	return "Err"

    cgroup_num = grep_results[start_index: end_index]
    return "memory:limitmem_" + cgroup_num

stop_mark = False
mem_usages = list()
max_mem_usage = 0

def cgroup_polling():
    global stop_mark
    global mem_usages
    global max_mem_usage

    while 1:
    	cgroup_name = get_cgroup_name()
    	if cgroup_name == "Err":
    		continue
    	break
    print "cgroup_name: " + cgroup_name

    while 1 and (not stop_mark):
    	time.sleep(0.01)
    	memusage_results = os.popen(CmdGetCgroupMemUsage['start_all'].format(cgroup_name=cgroup_name)).read()
        print memusage_results
    	cur_memusage = int(memusage_results.rstrip("\n").split()[1])
    	mem_usages.append(cur_memusage)

    	max_memusage_results = os.popen(CmdGetCgroupMaxMemUsage['start'].format(cgroup_name=cgroup_name)).read()
    	max_mem_usage = int(max_memusage_results.rstrip("\n").split()[1])

def kill_keyword(task):
    if "-ipsec" in task:
    	return task[0: -6]
    else:
    	return task

def run_limitmem(task, memsize):
    print colored("run_limitmem: task" + " " + str(memsize) + "KB", 'yellow')

    global stop_mark
    global mem_usages
    global max_mem_usage

    stop_mark = False
    mem_usages = list()
    max_mem_usage = 0

    polling = threading.Thread(target=cgroup_polling)
    polling.start()
    print "pooling starts"

    # we do not set limit to the process memory
    results = os.popen(CmdLimitMemBg['start'].format(mem=str(memsize), task=task)).read()
    print results

    stop_mark = True
    time.sleep(5) # wait for the port being restored.

    return 0


if __name__ == '__main__':
    now = datetime.datetime.now()
    limitmem_res = open("./examples/memory-profiling/cgroup-log/memusage.txt_" + now.isoformat(), 'w')
    tasks = ["acl-fw-ipsec", "dpi-ipsec", "lpm-ipsec", "maglev-ipsec", "nat-tcp-v4-ipsec"]

    for task in tasks:
    	res = run_limitmem(task, 4 * 1024 * 1024)
    	if res == -1:
    		print "retesting fails"
    	else:
    		print "retesting succeeds"

    	total_mem_usages = map(lambda x: x / (1024 * 1024.0), mem_usages)
    	max_total_mem_usages = max_mem_usage  / (1024 * 1024.0)
    	
    	# print total_mem_usages
    	print colored("[Cgroup direct]: peak_total_mem_usage: " + str(max_total_mem_usages), 'green')

    	limitmem_res.write(task + "," + pktgen_type + ",")
    	limitmem_res.write(str(max_total_mem_usages) + "\n")
    	limitmem_res.flush()

    limitmem_res.close()