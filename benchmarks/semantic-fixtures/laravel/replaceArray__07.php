# ID: src/Illuminate/Support/Str.php:1215
<?php
public static function substituteSequentially($search, $replace, $subject)
{
    if ($replace instanceof Traversable) {
        $replace = iterator_to_array($replace);
    }

    $segments = explode($search, $subject);

    // The text before the first placeholder is kept verbatim
    $result = array_shift($segments);

    foreach ($segments as $piece) {
        $next = array_shift($replace) ?? $search;
        $result .= self::toStringOr($next, $search).$piece;
    }

    return $result;
}
