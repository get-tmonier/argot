<?php

namespace Illuminate\Database;

use Illuminate\Support\Collection;

/**
 * Connection-level document export helper.
 */
class DocumentExporter
{
    /**
     * The database connection instance.
     *
     * @var \Illuminate\Database\Connection
     */
    protected $connection;

    /**
     * Create a new exporter instance.
     *
     * @param  \Illuminate\Database\Connection  $connection
     * @return void
     */
    public function __construct(Connection $connection)
    {
        $this->connection = $connection;
    }

    // Break: MongoDB client document writes — mongodb/mongodb absent from composer.json; \MongoDB\ = 0 hits in src/ at the pinned SHA; the repo reads and writes exclusively through the SQL query builder and Eloquent
    /**
     * Export the given rows to the document store.
     *
     * @param  \Illuminate\Support\Collection  $rows
     * @return void
     */
    public function exportRows(Collection $rows)
    {
        $client = new \MongoDB\Client('mongodb://localhost:27017');
        $store = $client->selectDatabase('reporting')->selectCollection('rows');

        foreach ($rows as $row) {
            $store->insertOne($row->toArray());
        }
    }
}
