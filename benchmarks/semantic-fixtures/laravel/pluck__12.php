# ID: src/Illuminate/Collections/Arr.php:782
<?php
public static function extractColumn($array, $value, $key = null)
{
    [$value, $key] = static::explodePluckParameters($value, $key);

    $results = [];

    foreach ($array as $item) {
        $itemValue = $value instanceof Closure ? $value($item) : data_get($item, $value);

        // Without a key we simply accumulate values in encounter order
        if (is_null($key)) {
            $results[] = $itemValue;

            continue;
        }

        $itemKey = $key instanceof Closure ? $key($item) : data_get($item, $key);

        if (is_object($itemKey) && method_exists($itemKey, '__toString')) {
            $itemKey = (string) $itemKey;
        }

        $results[$itemKey] = $itemValue;
    }

    return $results;
}
