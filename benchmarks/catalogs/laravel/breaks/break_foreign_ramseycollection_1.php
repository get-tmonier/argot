<?php

namespace Illuminate\Database;

use Illuminate\Support\Collection as SupportCollection;
use Ramsey\Collection\Collection;

/**
 * Connection-level typed row collector.
 */
class TypedRowCollector
{
    /**
     * The database connection instance.
     *
     * @var \Illuminate\Database\Connection
     */
    protected $connection;

    /**
     * Create a new collector instance.
     *
     * @param  \Illuminate\Database\Connection  $connection
     * @return void
     */
    public function __construct(Connection $connection)
    {
        $this->connection = $connection;
    }

    // Break: ramsey/collection typed collection — ramsey/collection absent from composer.json (require + require-dev); Ramsey\Collection has zero hits in src/ at the pinned SHA (only ramsey/uuid is present). The foreign class is named `Collection`, colliding with the attested Illuminate\Support\Collection the repo defines and calls, so the foreign construction is masked behind a repo-owned type name.
    /**
     * Collect the given rows into a strictly typed collection.
     *
     * @param  \Illuminate\Support\Collection  $rows
     * @return array
     */
    public function collectTyped(SupportCollection $rows)
    {
        $typed = new Collection('array');

        foreach ($rows as $row) {
            $typed->add($row->toArray());
        }

        return $typed->toArray();
    }
}
