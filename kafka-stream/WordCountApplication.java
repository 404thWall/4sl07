import org.apache.kafka.clients.consumer.ConsumerConfig;
import org.apache.kafka.clients.producer.ProducerConfig;
import org.apache.kafka.common.serialization.Serdes;
import org.apache.kafka.common.utils.Bytes;
import org.apache.kafka.streams.KafkaStreams;
import org.apache.kafka.streams.KeyValue;
import org.apache.kafka.streams.StreamsBuilder;
import org.apache.kafka.streams.StreamsConfig;
import org.apache.kafka.streams.kstream.Grouped;
import org.apache.kafka.streams.kstream.Materialized;
import org.apache.kafka.streams.kstream.Produced;
import org.apache.kafka.streams.state.KeyValueStore;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.net.InetAddress;
import java.net.URI;
import java.net.URL;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;
import java.util.Properties;
import java.util.zip.GZIPInputStream;

public class WordCountApplication {

    private static final String INPUT_TOPIC  = "Files";
    private static final String OUTPUT_TOPIC = "Maps";
    private static final String STORE_NAME   = "counts-store";
    private static final String CRAWL_URL    = "https://data.commoncrawl.org/";

    private static List<String> WET_PATHS;

    public static void main(final String[] args) throws Exception {
        WET_PATHS = Files.readAllLines(Paths.get("wet.paths"));

        Properties props = new Properties();
        props.put(StreamsConfig.APPLICATION_ID_CONFIG,           "wordcount-application");
        props.put(StreamsConfig.BOOTSTRAP_SERVERS_CONFIG,         args[0]);
        props.put(StreamsConfig.DEFAULT_KEY_SERDE_CLASS_CONFIG,   Serdes.String().getClass());
        props.put(StreamsConfig.DEFAULT_VALUE_SERDE_CLASS_CONFIG, Serdes.String().getClass());

        props.put(StreamsConfig.producerPrefix(ProducerConfig.DELIVERY_TIMEOUT_MS_CONFIG), 600_000); 
        props.put(StreamsConfig.producerPrefix(ProducerConfig.REQUEST_TIMEOUT_MS_CONFIG),  300_000); 
        props.put(StreamsConfig.producerPrefix(ProducerConfig.MAX_BLOCK_MS_CONFIG),        600_000);
        props.put(StreamsConfig.producerPrefix(ProducerConfig.BATCH_SIZE_CONFIG),          524_288);     
        props.put(StreamsConfig.producerPrefix(ProducerConfig.LINGER_MS_CONFIG),               200);         
        props.put(StreamsConfig.producerPrefix(ProducerConfig.BUFFER_MEMORY_CONFIG),  134_217_728L);
        props.put(StreamsConfig.producerPrefix(ProducerConfig.COMPRESSION_TYPE_CONFIG),      "lz4");

        props.put(StreamsConfig.consumerPrefix(ConsumerConfig.SESSION_TIMEOUT_MS_CONFIG),   120_000); // 2 min
        props.put(StreamsConfig.consumerPrefix(ConsumerConfig.MAX_POLL_INTERVAL_MS_CONFIG), 600_000); // 10 min
        props.put(StreamsConfig.consumerPrefix(ConsumerConfig.HEARTBEAT_INTERVAL_MS_CONFIG), 10_000); // 10 s
        props.put(StreamsConfig.consumerPrefix(ConsumerConfig.GROUP_INSTANCE_ID_CONFIG),
                  InetAddress.getLocalHost().getHostName());

        props.put(StreamsConfig.STATE_DIR_CONFIG, "/tmp/grp3-kafka-streams");

        StreamsBuilder builder = new StreamsBuilder();

        builder.<String, String>stream(INPUT_TOPIC)
            .flatMap((fileKey, indexStr) -> parseFile(Integer.parseInt(indexStr.trim())))
            .groupByKey(Grouped.with(Serdes.String(), Serdes.Long()))
            .reduce(
                Long::sum,
                Materialized.<String, Long, KeyValueStore<Bytes, byte[]>>as(STORE_NAME)
                    .withKeySerde(Serdes.String())
                    .withValueSerde(Serdes.Long())
            )
            .toStream()
            .to(OUTPUT_TOPIC, Produced.with(Serdes.String(), Serdes.Long()));

        KafkaStreams streams = new KafkaStreams(builder.build(), props);

        // Arrêt propre sur SIGTERM/SIGINT
        Runtime.getRuntime().addShutdownHook(new Thread(streams::close));

        streams.start();
    }

    private static List<KeyValue<String, Long>> parseFile(int index) {
        String path = WET_PATHS.get(index);
        String url  = CRAWL_URL + path;
        try {
            System.out.println("Start downloading file " + index);
            long start = System.currentTimeMillis();

            Map<String, Long> localCounts;
            try (GZIPInputStream gzip = new GZIPInputStream(URI.create(url).toURL().openStream());
                BufferedReader reader = new BufferedReader(new InputStreamReader(gzip))) {
                List<String> words = reader.lines()
                    .flatMap(line -> Arrays.stream(line.toLowerCase().split("\\W+")))
                    .filter(w -> !w.isEmpty())
                    .collect(Collectors.toList());
                long downloadMs = System.currentTimeMillis() - start;

                long mapStart = System.currentTimeMillis();
                localCounts = words.stream()
                    .collect(Collectors.groupingBy(w -> w, Collectors.counting()));
                long mapMs = System.currentTimeMillis() - mapStart;

                System.out.println("End downloading file " + index + " in " + downloadMs + "ms, map in " + mapMs + "ms");
            }

            return localCounts.entrySet().stream()
                .map(e -> KeyValue.pair(e.getKey(), e.getValue()))
                .collect(Collectors.toList());

        } catch (IOException e) {
            System.err.println("Error downloading " + url + ": " + e.getMessage());
            return Collections.emptyList();
        }
    }
}