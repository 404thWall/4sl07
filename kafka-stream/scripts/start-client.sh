trap 'kill 0' EXIT HUP TERM INT

if [ $# -ne 1 ]; then
    echo "Usage: $0 <broker_host:port>"
    exit 1
fi

CLIENT=$1

rm -rf /tmp/grp3-kafka-streams
pkill -f WordCountApplication; pkill -f DirectoryProducer; pkill -f KafkaWatcher
java -cp "kafka/libs/*":libs/ WordCountApplication $CLIENT