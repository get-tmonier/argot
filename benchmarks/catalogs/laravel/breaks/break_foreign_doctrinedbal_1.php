<?php

namespace Illuminate\Database;

use Doctrine\DBAL\Connection;

/**
 * Reporting reader that runs against a raw DBAL connection.
 */
class DbalReportReader
{
    /**
     * Format a report row for output.
     *
     * @param  array  $row
     * @return array
     */
    protected function formatRow(array $row)
    {
        return array_change_key_case($row, CASE_LOWER);
    }

    // Break: doctrine/dbal raw connection — doctrine/dbal absent from composer.json (require + require-dev); Doctrine\DBAL has zero hits in src/ at the pinned SHA (only doctrine/inflector is present). The foreign dependency is reached purely through a receiver variable whose type is the ambient short name `Connection` — colliding with the attested Illuminate\Database\Connection — and whose leaf methods (executeQuery/fetchAllAssociative) never name a foreign namespace, so no foreign symbol is visible in the call graph.
    /**
     * Run the given report query and return the formatted rows.
     *
     * @param  \Doctrine\DBAL\Connection  $dbal
     * @param  string  $sql
     * @return array
     */
    public function runReport(Connection $dbal, $sql)
    {
        $result = $dbal->executeQuery($sql);

        $rows = $result->fetchAllAssociative();

        return array_map([$this, 'formatRow'], $rows);
    }
}
