#!/bin/bash
echo "============================================="
echo "🔧  Distributed Qiyana-wosel System (Worker Num: ${WORKER_NUM:-8})"
echo "============================================="

WORKER_NUM=${WORKER_NUM:-8} # 2^a
WORKER_THREADS=16
echo "Using $WORKER_NUM workers"

rm -f ./result/result_master.txt ./result/result_workers.txt ./result/result_client.txt 2>/dev/null

echo -e "\n1. Starting components sequentially..."

MASTER_CORE=0
CLIENT_CORE=64

pkill -f "qiyanawoselmaster\|qiyanawoselworker\|qiyanawoselclient" 2>/dev/null || true

echo -e "\n📱  Starting Master Node..."
RUSTFLAGS="-C target-cpu=native" numactl --cpunodebind=0 --membind=1 taskset -c $MASTER_CORE cargo run --release --bin qiyanawoselmaster -- $WORKER_NUM > ./result/result_master.txt 2>&1 &
MASTER_PID=$!
echo "Master PID: $MASTER_PID"
sleep 3


> ./result/result_workers.txt
WORKERS_PER_NUMA=$((WORKER_NUM / 3))
REMAINDER=$((WORKER_NUM % 3))
for ((i=0; i<WORKER_NUM; i++)); do
    echo -e "\n👷  Starting Worker $i..."
    if [ $i -lt $WORKERS_PER_NUMA ]; then
        NUMA_NODE=0
        START_BASE=1
        NODE_CORES=63
    elif [ $i -lt $((WORKERS_PER_NUMA * 2)) ]; then
        NUMA_NODE=1
        START_BASE=65
        NODE_CORES=127
    else
        NUMA_NODE=2
        START_BASE=128
        NODE_CORES=191
    fi

    if [ $NUMA_NODE -eq 0 ]; then
        NODE_INDEX=$i
    elif [ $NUMA_NODE -eq 1 ]; then
        NODE_INDEX=$((i - WORKERS_PER_NUMA))
    else
        NODE_INDEX=$((i - WORKERS_PER_NUMA * 2))
    fi
    
    START_CORE=$((START_BASE + NODE_INDEX * WORKER_THREADS))
    END_CORE=$((START_CORE + WORKER_THREADS - 1))
    
    if [ $END_CORE -gt $NODE_CORES ]; then
        echo "⚠️  Warning: Worker $i core range $START_CORE-$END_CORE exceeds NUMA node $NUMA_NODE range"
        START_CORE=$START_BASE
        END_CORE=$((START_BASE + WORKER_THREADS - 1))
    fi

    CORE_RANGE="${START_CORE}-${END_CORE}"
    
    RUSTFLAGS="-C target-cpu=native" numactl --cpunodebind=$NUMA_NODE --membind=$NUMA_NODE taskset -c $CORE_RANGE cargo run --release --bin qiyanawoselworker -- $i $WORKER_NUM >> ./result/result_workers.txt 2>&1 &
    WORKER_PIDS[$i]=$!
    echo "Worker $i PID: ${WORKER_PIDS[$i]}"
done

sleep 1
echo -e "\n🔍  Starting Client..."
RUSTFLAGS="-C target-cpu=native" numactl --cpunodebind=1 --membind=1 taskset -c $CLIENT_CORE cargo run --release --bin qiyanawoselclient -- $WORKER_NUM 2>&1 | tee ./result/result_client.txt &
CLIENT_PID=$!
echo "Client PID: $CLIENT_PID"

echo -e "\n📊   Log files created:"
echo "   Master: ./result/result_master.txt ($(wc -l < ./result/result_master.txt) lines)"
echo "   Workers: ./result/result_workers.txt ($(wc -l < ./result/result_workers.txt) lines)"
echo "   Client: ./result/result_client.txt ($(wc -l < ./result/result_client.txt) lines)"

echo -e "\nPress Ctrl+C to stop all processes..."
echo "Waiting for processes to complete..."

wait
echo -e "\n✅  Test completed!"
