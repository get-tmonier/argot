<?php

namespace Illuminate\Database;

use Elasticsearch\ClientBuilder;
use Illuminate\Support\Collection;

/**
 * Connection-level search index writer.
 */
class SearchIndexer
{
    /**
     * The database connection instance.
     *
     * @var \Illuminate\Database\Connection
     */
    protected $connection;

    /**
     * Create a new indexer instance.
     *
     * @param  \Illuminate\Database\Connection  $connection
     * @return void
     */
    public function __construct(Connection $connection)
    {
        $this->connection = $connection;
    }

    // Break: elasticsearch/elasticsearch index writes — elasticsearch/elasticsearch absent from composer.json (require + require-dev); Elasticsearch\ClientBuilder has zero hits in src/ at the pinned SHA. The builder is reached by short name through a receiver variable, not a fully-qualified call; the repo queries exclusively through the SQL query builder and Eloquent.
    /**
     * Push the given rows into the search index.
     *
     * @param  \Illuminate\Support\Collection  $rows
     * @return void
     */
    public function indexRows(Collection $rows)
    {
        $builder = new ClientBuilder();

        $client = $builder->setHosts(['http://localhost:9200'])->build();

        foreach ($rows as $row) {
            $client->index([
                'index' => 'records',
                'body' => $row->toArray(),
            ]);
        }
    }
}
