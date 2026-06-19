import org.apache.kafka.clients.admin.*;
import org.apache.kafka.clients.consumer.OffsetAndMetadata;
import org.apache.kafka.clients.producer.KafkaProducer;
import org.apache.kafka.clients.producer.ProducerConfig;
import org.apache.kafka.clients.producer.ProducerRecord;
import org.apache.kafka.common.TopicPartition;
import org.apache.kafka.common.TopicPartitionInfo;
import org.apache.kafka.common.serialization.StringSerializer;

import java.util.*;

public class Orchestrator {
    public static void main(String[] args) throws Exception {
        if (args.length < 1) {
            System.out.println("Usage: java Orchestrator <n>");
            return;
        }

        int    n       = Integer.parseInt(args[0]);
        String topic   = "Files";
        String groupId = "wordcount-application";

        Properties producerProps = new Properties();
        producerProps.put(ProducerConfig.BOOTSTRAP_SERVERS_CONFIG,        "localhost:9092");
        producerProps.put(ProducerConfig.KEY_SERIALIZER_CLASS_CONFIG,     StringSerializer.class.getName());
        producerProps.put(ProducerConfig.VALUE_SERIALIZER_CLASS_CONFIG,   StringSerializer.class.getName());
        producerProps.put(ProducerConfig.ENABLE_IDEMPOTENCE_CONFIG,       "false");

        Properties adminProps = new Properties();
        adminProps.put("bootstrap.servers", "localhost:9092");

        // ── Step 1: send indices ─────────────────────────────────────────────
        long sendStart = System.currentTimeMillis();
        int  sent      = 0;

        try (KafkaProducer<String, String> producer = new KafkaProducer<>(producerProps)) {
            for (int i = 1; i < n; i++) {
                producer.send(new ProducerRecord<>(topic, String.valueOf(i), String.valueOf(i)));
                sent++;
            }
        }

        long sendEnd = System.currentTimeMillis();
        System.out.printf("Send complete: %d files in %d ms%n", sent, sendEnd - sendStart);

        // ── Step 2: wait for processing to complete ──────────────────────────
        System.out.println("Waiting for workers to finish...");
        long processStart = System.currentTimeMillis();

        try (AdminClient admin = AdminClient.create(adminProps)) {
            // Fetch topic partitions
            Set<TopicPartition> topicPartitions = new HashSet<>();
            List<TopicPartitionInfo> partitions = admin
                    .describeTopics(Collections.singleton(topic))
                    .topicNameValues().get(topic).get()
                    .partitions();
            for (TopicPartitionInfo p : partitions)
                topicPartitions.add(new TopicPartition(topic, p.partition()));

            while (true) {
                Map<TopicPartition, OffsetAndMetadata> groupOffsets = admin
                        .listConsumerGroupOffsets(groupId)
                        .partitionsToOffsetAndMetadata().get();

                Map<TopicPartition, OffsetSpec> request = new HashMap<>();
                for (TopicPartition tp : topicPartitions) request.put(tp, OffsetSpec.latest());
                Map<TopicPartition, ListOffsetsResult.ListOffsetsResultInfo> endOffsets = admin
                        .listOffsets(request).all().get();

                long totalLag = 0;
                for (TopicPartition tp : topicPartitions) {
                    long current = groupOffsets.getOrDefault(tp, new OffsetAndMetadata(0)).offset();
                    long end     = endOffsets.get(tp).offset();
                    totalLag    += Math.max(0, end - current);
                }

                long elapsed = System.currentTimeMillis() - processStart;
                System.out.printf("[%6d s] Remaining lag: %d files%n", elapsed / 1000, totalLag);

                if (totalLag == 0 && sent > 0) break;
                Thread.sleep(10_000);
            }
        }

        long totalTime = System.currentTimeMillis() - sendStart;
        System.out.printf("Done: %d files processed in %d ms (%.1f s)%n",
                sent, totalTime, totalTime / 1000.0);
    }
}
