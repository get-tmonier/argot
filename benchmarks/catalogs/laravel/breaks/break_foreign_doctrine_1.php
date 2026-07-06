<?php

namespace Illuminate\Database;

use Illuminate\Support\Collection;

/**
 * Connection-level bulk record synchronizer.
 */
class RecordSynchronizer
{
    /**
     * The database connection instance.
     *
     * @var \Illuminate\Database\Connection
     */
    protected $connection;

    /**
     * Create a new synchronizer instance.
     *
     * @param  \Illuminate\Database\Connection  $connection
     * @return void
     */
    public function __construct(Connection $connection)
    {
        $this->connection = $connection;
    }

    // Break: Doctrine ORM EntityManager unit-of-work — doctrine/orm absent from composer.json (require + require-dev); \Doctrine\ORM = 0 hits in src/ at the pinned SHA; the repo persists through Eloquent models and the query builder, never a foreign ORM
    /**
     * Persist the given entities to storage.
     *
     * @param  \Illuminate\Support\Collection  $entities
     * @return void
     */
    public function persistEntities(Collection $entities)
    {
        $config = \Doctrine\ORM\ORMSetup::createAttributeMetadataConfiguration([__DIR__], true);
        $manager = \Doctrine\ORM\EntityManager::create($this->connection->getPdo(), $config);

        foreach ($entities as $entity) {
            $manager->persist($entity);
        }

        $manager->flush();
        $manager->clear();
    }
}
