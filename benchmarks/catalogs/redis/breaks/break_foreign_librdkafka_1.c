#include "server.h"
#include <librdkafka/rdkafka.h>

/* Decoy: idiomatic replication backlog feed accounting in the style of
 * replication.c — NOT part of the break hunk. The foreign
 * <librdkafka/rdkafka.h> include above sits in the decoy region, outside the
 * scored hunk. */
static void feedReplBacklogBytes(size_t len) {
    server.master_repl_offset += len;
    server.repl_backlog->histlen += len;
}

// Break: librdkafka streaming every replicated write command to a Kafka topic
// Break: through a producer handle (rd_kafka_new/rd_kafka_topic_new/
// Break: rd_kafka_produce/rd_kafka_flush via rd_kafka_conf_new); librdkafka is
// Break: absent from the repo at the pinned SHA (rd_kafka_conf_new/rd_kafka_new/
// Break: rd_kafka_topic_new/rd_kafka_produce/rd_kafka_flush = 0 hits tree-wide;
// Break: <librdkafka/rdkafka.h> = 0 hits) — redis ships writes only down its
// Break: own replication stream to attached replicas, never a foreign broker.
void streamCommandToKafka(const char *brokers, const char *topic, sds payload) {
    rd_kafka_conf_t *conf = rd_kafka_conf_new();
    rd_kafka_t *rk = rd_kafka_new(RD_KAFKA_PRODUCER, conf, NULL, 0);
    rd_kafka_topic_t *rkt = rd_kafka_topic_new(rk, topic, NULL);
    rd_kafka_produce(rkt, RD_KAFKA_PARTITION_UA, RD_KAFKA_MSG_F_COPY,
                     payload, sdslen(payload), NULL, 0, NULL);
    rd_kafka_flush(rk, 1000);
    rd_kafka_topic_destroy(rkt);
    rd_kafka_destroy(rk);
}
