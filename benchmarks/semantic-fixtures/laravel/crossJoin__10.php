# ID: src/Illuminate/Collections/Arr.php:142
<?php
public static function cartesianProduct(...$arrays)
{
    $results = [[]];

    foreach ($arrays as $index => $array) {
        $expanded = [];

        foreach ($results as $product) {
            foreach ($array as $item) {
                $product[$index] = $item;
                $expanded[] = $product;
            }
        }

        $results = $expanded;
    }

    return $results;
}
