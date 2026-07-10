# ID: src/Illuminate/Collections/Collection.php:575
<?php
public static function indexByKey($collection, $keyBy)
{
    $keyBy = $collection->valueRetriever($keyBy);

    $results = [];

    foreach ($collection->items as $key => $item) {
        $resolvedKey = $keyBy($item, $key);

        if ($resolvedKey instanceof \UnitEnum) {
            $resolvedKey = enum_value($resolvedKey);
        }

        if (is_object($resolvedKey)) {
            $resolvedKey = (string) $resolvedKey;
        }

        $results[$resolvedKey] = $item;
    }

    return $collection->newInstance($results);
}
