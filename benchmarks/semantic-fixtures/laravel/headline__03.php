# ID: src/Illuminate/Support/Str.php:1462
<?php
public static function toHeadline($value)
{
    $words = preg_split('/\s+/u', $value, -1, PREG_SPLIT_NO_EMPTY);

    if (count($words) > 1) {
        $words = array_map(static::title(...), $words);
    } else {
        // A single token may still be camelCase, so split on upper-case boundaries
        $words = array_map(static::title(...), static::ucsplit(implode('_', $words)));
    }

    $normalized = static::replace(['-', '_', ' '], '_', implode('_', $words));

    return implode(' ', array_filter(explode('_', $normalized)));
}
