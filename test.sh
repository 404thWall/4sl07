#!/bin/bash

echo "Killing previous processes..."
while IFS= read -r line; do
    kill $line 2>/dev/null
done < pids.txt

if [ $# -gt 0 ]; then
    exit 0
fi

rm -rf ./tests/client_*/
rm -rf ./tests/server/
rm -rf ./tests/result/
cargo build --release
mkdir -p ./tests/server
cp ./target/release/slr07 ./tests/server/
cd ./tests/server/
./slr07 server 2>&1 > >(tee ./server.log) & PID_serv=$!
cd ../..

echo "Test (on a lancé $PID_serv)"
echo "$PID_serv" > pids.txt

N=3
for i in $(seq 1 $N)
do
    PORT=$((9000 + $i))
    mkdir ./tests/client_$i/
    cp ./target/release/slr07 ./tests/client_$i/
    cd ./tests/client_$i/
    ./slr07 client $PORT 127.0.0.1 test 2>&1 > >(tee ./client_$i.log) & PID_client=$!
    echo "Client $i (PID: $PID_client) connecté au port $PORT"
    cd ../..
    echo "$PID_client" >> pids.txt
done





# cargo run --release 2>&1 | tee /chemin/que/tu/choisis/app.log
