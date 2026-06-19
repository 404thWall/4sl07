IP=$(hostname -I | awk '{print $1}')
sed -i "s|advertised.listeners=PLAINTEXT://[^:]*:9092,CONTROLLER://[^:]*:9093|advertised.listeners=PLAINTEXT://$IP:9092,CONTROLLER://$IP:9093|" kafka/config/server.properties
