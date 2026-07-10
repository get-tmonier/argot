# ID: src/Illuminate/Collections/Collection.php:1378
<?php
public static function divideIntoGroups($collection, $numberOfGroups)
{
    if ($numberOfGroups < 1) {
        throw new InvalidArgumentException('Number of groups must be at least 1.');
    }

    if ($collection->isEmpty()) {
        return $collection->newInstance();
    }

    $total = $collection->count();
    $baseSize = floor($total / $numberOfGroups);
    $leftover = $total % $numberOfGroups;

    $groups = $collection->newInstance();
    $offset = 0;

    for ($group = 0; $group < $numberOfGroups; $group++) {
        // The first "leftover" groups each receive one extra item
        $size = $group < $leftover ? $baseSize + 1 : $baseSize;

        if ($size) {
            $groups->push($collection->newInstance(array_slice($collection->items, $offset, $size)));
            $offset += $size;
        }
    }

    return $groups;
}
