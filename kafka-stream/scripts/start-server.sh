trap 'kill 0' EXIT HUP TERM INT

JAVA_FILES="WordCountApplication.java Orchestrator.java"
javac -cp "kafka/libs/*" -d libs/ $JAVA_FILES

kafka/bin/kafka-server-stop.sh
sleep 3
rm -rf /tmp/log_grp3/kraft-combined-logs
KAFKA_CLUSTER_ID="$(kafka/bin/kafka-storage.sh random-uuid)" 
kafka/bin/kafka-storage.sh format --standalone -t $KAFKA_CLUSTER_ID -c kafka/config/server.properties > logs/format.log
kafka/bin/kafka-server-start.sh kafka/config/server.properties > logs/server.log &
sleep 10
kafka/bin/kafka-topics.sh --create --if-not-exists --topic wordcount-application-counts-store-changelog --partitions 100 --replication-factor 1 --bootstrap-server localhost:9092
kafka/bin/kafka-topics.sh --create --if-not-exists --topic Files --partitions 100 --replication-factor 1 --bootstrap-server localhost:9092
kafka/bin/kafka-topics.sh --create --if-not-exists --topic Maps  --partitions 20 --replication-factor 1 --bootstrap-server localhost:9092
java -cp "kafka/libs/*":libs/ Orchestrator $1 > logs/orchestrator.log