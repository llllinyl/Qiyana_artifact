#!/bin/bash

echo "============================================="
echo "🔧  Client-to-Server Qiyana-wosel Test (With Logs)"
echo "============================================="

rm -f ./result/result_single_server.txt ./result/result_single_client.txt 2>/dev/null

pkill -f "qiyana1_server\|qiyana1_client" 2>/dev/null || true

echo -e "\n1. Starting Server (logs to ./result/result_single_server.txt)..."
RUSTFLAGS="-C target-cpu=native" cargo run --release --bin qiyana1_server > ./result/result_single_server.txt 2>&1 &
SERVER_PID=$!
echo "   Server PID: $SERVER_PID"
echo "   Server logs: ./result/result_single_server.txt"

echo -n "   Waiting for server to start"
sleep 2
echo

echo -e "\n2. Starting Client (logs to ./result/result_single_client.txt)..."
echo "================ CLIENT OUTPUT ================"
RUSTFLAGS="-C target-cpu=native" cargo run --release --bin qiyana1_client 2>&1 | tee ./result/result_single_client.txt
CLIENT_EXIT=$?
echo "================ CLIENT ENDED ================="

echo -e "\n3. Stopping Server..."
kill $SERVER_PID 2>/dev/null || true
wait $SERVER_PID 2>/dev/null

echo -e "\n📊  Log files created:"
echo "   Server: ./result/result_single_server.txt ($(wc -l < ./result/result_single_server.txt) lines)"
echo "   Client: ./result/result_single_client.txt ($(wc -l < ./result/result_single_client.txt) lines)"

echo -e "\nPress Ctrl+C to stop all processes..."
echo "Waiting for processes to complete..."

wait
echo -e "\n✅ Test completed!"