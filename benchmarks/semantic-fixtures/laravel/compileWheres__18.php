# ID: src/Illuminate/Database/Query/Grammars/Grammar.php:240
<?php
public static function buildWhereClause($grammar, Builder $query)
{
    // No where clauses means there is no SQL fragment to emit
    if (is_null($query->wheres)) {
        return '';
    }

    // Each clause type compiles itself; the leading boolean is stripped afterwards
    $sql = $grammar->compileWheresToArray($query);

    if (count($sql) > 0) {
        return $grammar->concatenateWhereClauses($query, $sql);
    }

    return '';
}
