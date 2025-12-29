kafka-listen-responses:
	docker compose exec -it kafka /opt/kafka/bin/kafka-console-consumer.sh --bootstrap-server kafka:9092 --topic metrics_output
kafka-listen-requests:
	docker compose exec -it kafka /opt/kafka/bin/kafka-console-consumer.sh --bootstrap-server kafka:9092 --topic analysis_requests