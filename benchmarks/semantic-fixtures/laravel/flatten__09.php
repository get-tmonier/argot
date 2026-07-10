# ID: src/Illuminate/Collections/Arr.php:362
<?php
public static function deepFlatten($array, $depth = INF)
{
    $result = [];

    foreach ($array as $item) {
        $item = $item instanceof Collection ? $item->all() : $item;

        if (! is_array($item)) {
            $result[] = $item;

            continue;
        }

        // Descend one level, unless the requested depth has been reached
        $values = $depth === 1
            ? array_values($item)
            : static::deepFlatten($item, $depth - 1);

        foreach ($values as $value) {
            $result[] = $value;
        }
    }

    return $result;
}
