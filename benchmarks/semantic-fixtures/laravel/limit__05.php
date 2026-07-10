# ID: src/Illuminate/Support/Str.php:742
<?php
public static function truncate($value, $limit = 100, $end = '...', $preserveWords = false)
{
    if (mb_strwidth($value, 'UTF-8') <= $limit) {
        return $value;
    }

    if (! $preserveWords) {
        return rtrim(mb_strimwidth($value, 0, $limit, '', 'UTF-8')).$end;
    }

    // Flatten line breaks and drop tags before we hunt for word boundaries
    $value = trim(preg_replace('/[\n\r]+/', ' ', strip_tags($value)));

    $clipped = rtrim(mb_strimwidth($value, 0, $limit, '', 'UTF-8'));

    // A cut that lands exactly on a space can keep the whole clip
    if (mb_substr($value, $limit, 1, 'UTF-8') === ' ') {
        return $clipped.$end;
    }

    return preg_replace("/(.*)\s.*/", '$1', $clipped).$end;
}
