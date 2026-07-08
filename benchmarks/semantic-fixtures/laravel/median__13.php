# ID: src/Illuminate/Collections/Collection.php:98
<?php
public static function middleValue($collection, $key = null)
{
    $values = (isset($key) ? $collection->pluck($key) : $collection)
        ->reject(fn ($item) => is_null($item))
        ->sort()->values();

    $count = $values->count();

    if ($count === 0) {
        return;
    }

    $middle = intdiv($count, 2);

    // An odd count has a single central element
    if ($count % 2) {
        return $values->get($middle);
    }

    // An even count averages the two central elements
    return $collection->newInstance([
        $values->get($middle - 1), $values->get($middle),
    ])->average();
}
