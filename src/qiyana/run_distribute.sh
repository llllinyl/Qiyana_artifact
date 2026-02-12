#!/bin/bash
echo "============================================="
echo "🔧 Distributed Qiyana System (Worker Num: ${WORKER_NUM:-2})"
echo "============================================="

WORKER_NUM=${WORKER_NUM:-2} # 2^a
MODE=${MODE:-1}
echo "Using $WORKER_NUM workers"

if [ "$MODE" == "0" ]; then
    echo "Using standard communication"
fi

if [ "$MODE" == "1" ]; then
    echo "Using compressed communication"
fi


rm -f ./result/result_master.txt ./result/result_workers.txt ./result/result_client.txt 2>/dev/null

echo -e "\n1. Starting components sequentially..."

pkill -f "qiyanamaster\|qiyanaworker\|qiyanaclient" 2>/dev/null || true

echo -e "\n📱 Starting Master Node..."
RUSTFLAGS="-C target-cpu=native" cargo run --release --bin qiyanamaster -- $WORKER_NUM > ./result/result_master.txt 2>&1 &
MASTER_PID=$!
echo "Master PID: $MASTER_PID"
sleep 3


> ./result/result_workers.txt

for ((i=0; i<WORKER_NUM; i++)); do
    echo -e "\n👷 Starting Worker $i..."
    RUSTFLAGS="-C target-cpu=native" cargo run --release --bin qiyanaworker -- $i $WORKER_NUM $MODE >> ./result/result_workers.txt 2>&1 &
    WORKER_PIDS[$i]=$!
    echo "Worker $i PID: ${WORKER_PIDS[$i]}"
done

sleep 1
echo -e "\n🔍 Starting Client..."
RUSTFLAGS="-C target-cpu=native" cargo run --release --bin qiyanaclient -- $WORKER_NUM $MODE 2>&1 | tee ./result/result_client.txt &
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