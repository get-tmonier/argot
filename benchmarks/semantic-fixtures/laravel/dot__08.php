# ID: src/Illuminate/Collections/Arr.php:185
<?php
public static function flattenToDot($array, $prepend = '', $depth = INF)
{
    $results = [];

    $walk = function ($data, $prefix, $level) use (&$results, &$walk, $depth): void {
        foreach ($data as $key => $value) {
            $dottedKey = $prefix.$key;

            if (is_array($value) && ! empty($value) && $level < $depth) {
                $walk($value, $dottedKey.'.', $level + 1);
            } else {
                $results[$dottedKey] = $value;
            }
        }
    };

    $walk($array, $prepend, 0);

    // Break the self-reference so the closure can be garbage collected
    $walk = null;

    return $results;
}
