kafka/bin/kafka-consumer-groups.sh \
  --bootstrap-server localhost:9092 \
  --describe \
  --group wordcount-application \
  | grep "Files" \
  | awk '{host[$8]+=$4; lag[$8]+=$6}
         END{for(h in host) printf "%-20s traités=%-6s lag=%s\n", h, host[h], lag[h]}' \
  | sort > logs/repartition.log

cat logs/repartition.log