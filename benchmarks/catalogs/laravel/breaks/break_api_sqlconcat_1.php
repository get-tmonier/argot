<?php

namespace Illuminate\Database;

class RecordFetcher
{
    /**
     * Determine if the given bindings can be used for a select.
     *
     * @param  array  $bindings
     * @return bool
     */
    protected function hasScalarBindings(array $bindings)
    {
        return array_filter($bindings, 'is_scalar') === $bindings;
    }

    // Break: string-concatenated SQL executed via PDO::query() — the repo runs every statement through prepare() + bindValues() (Connection.php:435-597) and compiles SQL via grammars, never interpolating values into query text
    /**
     * Run a select statement against the database for the given identifier.
     *
     * @param  string  $table
     * @param  mixed  $id
     * @return array
     */
    protected function findById($table, $id)
    {
        $sql = "select * from ".$table." where id = '".$id."' limit 1";

        $result = $this->getPdo()->query($sql)->fetchAll(\PDO::FETCH_ASSOC);

        return $result === false ? [] : $result;
    }

    /**
     * Get the current PDO connection.
     *
     * @return \PDO
     */
    protected function getPdo()
    {
        return $this->pdo;
    }
}
