#!/bin/bash
echo "============================================="
echo "🔧 Distributed Qiyana-wosel System (Worker Num: ${WORKER_NUM:-2})"
echo "============================================="

WORKER_NUM=${WORKER_NUM:-2} # 2^a
echo "Using $WORKER_NUM workers"

rm -f ./result/result_master.txt ./result/result_workers.txt ./result/result_client.txt 2>/dev/null

echo -e "\n1. Starting components sequentially..."

pkill -f "qiyanawoselmaster\|qiyanawoselworker\|qiyanawoselclient" 2>/dev/null || true

echo -e "\n📱 Starting Master Node..."
RUSTFLAGS="-C target-cpu=native" cargo run --release --bin qiyanawoselmaster -- $WORKER_NUM > ./result/result_master.txt 2>&1 &
MASTER_PID=$!
echo "Master PID: $MASTER_PID"
sleep 3


> ./result/result_workers.txt

for ((i=0; i<WORKER_NUM; i++)); do
    echo -e "\n👷 Starting Worker $i..."
    RUSTFLAGS="-C target-cpu=native" cargo run --release --bin qiyanawoselworker -- $i $WORKER_NUM >> ./result/result_workers.txt 2>&1 &
    WORKER_PIDS[$i]=$!
    echo "Worker $i PID: ${WORKER_PIDS[$i]}"
done

sleep 1
echo -e "\n🔍 Starting Client..."
RUSTFLAGS="-C target-cpu=native" cargo run --release --bin qiyanawoselclient -- $WORKER_NUM 2>&1 | tee ./result/result_client.txt &
CLIENT_PID=$!
echo "Client PID: $CLIENT_PID"

echo -e "\n📊  Log files created:"
echo "   Master: ./result/result_master.txt ($(wc -l < ./result/result_master.txt) lines)"
echo "   Workers: ./result/result_workers.txt ($(wc -l < ./result/result_workers.txt) lines)"
echo "   Client: ./result/result_client.txt ($(wc -l < ./result/result_client.txt) lines)"

echo -e "\nPress Ctrl+C to stop all processes..."
echo "Waiting for processes to complete..."

wait
echo -e "\n✅ Test completed!"