# ID: src/Illuminate/Support/Str.php:1597
<?php
public static function toSnakeCase($value, $delimiter = '_')
{
    $cacheKey = $value;

    if (isset(static::$snakeCache[$cacheKey][$delimiter])) {
        return static::$snakeCache[$cacheKey][$delimiter];
    }

    if (! ctype_lower($value)) {
        // Capitalize each word and drop the whitespace between them
        $value = preg_replace('/\s+/u', '', ucwords($value));

        // Insert the delimiter before every upper-case boundary, then lower-case
        $value = static::lower(preg_replace('/(.)(?=[A-Z])/u', '$1'.$delimiter, $value));
    }

    return static::$snakeCache[$cacheKey][$delimiter] = $value;
}
