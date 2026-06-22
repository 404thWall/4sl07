trap 'kill 0' EXIT HUP TERM INT

KAFKA_VERSION="2.13-4.3.0"
KAFKA_DIR="kafka_${KAFKA_VERSION}"
KAFKA_TGZ_URL="https://www.apache.org/dyn/closer.lua/kafka/4.3.0/kafka_2.13-4.3.0.tgz?action=download"

ensure_kafka() {
	if [ -d "kafka/libs" ] && [ -x "kafka/bin/kafka-server-start.sh" ]; then
		return 0
	fi

	echo "Kafka runtime not found under ./kafka. Downloading ${KAFKA_DIR}..."
	rm -rf "${KAFKA_DIR}" kafka kafka.tgz
	if command -v curl >/dev/null 2>&1; then
		curl -L --retry 5 --retry-delay 3 "$KAFKA_TGZ_URL" -o kafka.tgz
	elif command -v wget >/dev/null 2>&1; then
		wget -O kafka.tgz "$KAFKA_TGZ_URL"
	else
		echo "ERROR: neither curl nor wget is available to download Kafka"
		exit 1
	fi

	tar -xzf kafka.tgz
	mv "${KAFKA_DIR}" kafka
	rm -f kafka.tgz
	echo "Kafka downloaded and extracted to ./kafka"
}

mkdir -p logs libs
ensure_kafka

ensure_wet_paths() {
	if [ -f "wet.paths" ]; then
		return 0
	fi

	echo "wet.paths not found. Downloading..."
	if command -v curl >/dev/null 2>&1; then
		curl -L --retry 5 --retry-delay 3 \
			"https://data.commoncrawl.org/crawl-data/CC-MAIN-2023-14/wet.paths.gz" \
			-o wet.paths.gz
	elif command -v wget >/dev/null 2>&1; then
		wget -O wet.paths.gz \
			"https://data.commoncrawl.org/crawl-data/CC-MAIN-2023-14/wet.paths.gz"
	else
		echo "ERROR: neither curl nor wget is available to download wet.paths.gz"
		exit 1
	fi

	gunzip -f wet.paths.gz
	echo "wet.paths downloaded"
}

ensure_wet_paths

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