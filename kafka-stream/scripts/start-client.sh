trap 'kill 0' EXIT HUP TERM INT

if [ $# -ne 1 ]; then
    echo "Usage: $0 <broker_host:port>"
    exit 1
fi

CLIENT=$1

KAFKA_VERSION="2.13-4.3.0"
KAFKA_DIR="kafka_${KAFKA_VERSION}"
KAFKA_TGZ_URL="https://www.apache.org/dyn/closer.lua/kafka/4.3.0/kafka_2.13-4.3.0.tgz?action=download"

ensure_kafka() {
    if [ -d "kafka/libs" ] && [ -f "kafka/libs/kafka-clients-4.3.0.jar" ]; then
        return 0
    fi

    echo "Kafka libs not found under ./kafka. Downloading ${KAFKA_DIR}..."
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

mkdir -p libs
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

# Compile if classes are missing on this host.
if [ ! -f "libs/WordCountApplication.class" ] || [ ! -f "libs/Orchestrator.class" ]; then
    javac -cp "kafka/libs/*" -d libs/ WordCountApplication.java Orchestrator.java
fi

rm -rf /tmp/grp3-kafka-streams
pkill -f WordCountApplication; pkill -f DirectoryProducer; pkill -f KafkaWatcher
java -cp "kafka/libs/*":libs/ WordCountApplication $CLIENT