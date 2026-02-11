#!/bin/bash
echo "============================================="
echo "🔧 Distributed System Test (Worker Num: ${WORKER_NUM:-2})"
echo "============================================="

WORKER_NUM=${WORKER_NUM:-2} # 2^a
echo "Using $WORKER_NUM workers"

echo -e "\n1. Starting components sequentially..."

pkill -f "qiyanawoselmaster\|qiyanawoselworker\|qiyanawoselclient" 2>/dev/null || true

echo -e "\n📱 Starting Master Node..."
RUSTFLAGS="-C target-cpu=native" cargo run --release --bin qiyanawoselmaster -- $WORKER_NUM &
MASTER_PID=$!
echo "Master PID: $MASTER_PID"
sleep 3

for ((i=0; i<WORKER_NUM; i++)); do
    echo -e "\n👷 Starting Worker $i..."
    RUSTFLAGS="-C target-cpu=native" cargo run --release --bin qiyanawoselworker -- $i $WORKER_NUM &
    WORKER_PIDS[$i]=$!
    echo "Worker $i PID: ${WORKER_PIDS[$i]}"
done

sleep 1
echo -e "\n🔍 Starting Client..."
RUSTFLAGS="-C target-cpu=native" cargo run --release --bin qiyanawoselclient -- $WORKER_NUM &
CLIENT_PID=$!
echo "Client PID: $CLIENT_PID"

echo -e "\nPress Ctrl+C to stop all processes..."
echo "Waiting for processes to complete..."

wait
echo -e "\n✅ Test completed!"