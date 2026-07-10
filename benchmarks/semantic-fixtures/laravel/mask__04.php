# ID: src/Illuminate/Support/Str.php:852
<?php
public static function obscure($string, $character, $index, $length = null, $encoding = 'UTF-8')
{
    if ($character === '') {
        return $string;
    }

    $segment = mb_substr($string, $index, $length, $encoding);

    if ($segment === '') {
        return $string;
    }

    // Resolve a possibly-negative index into a concrete starting offset
    $startIndex = $index;
    if ($index < 0) {
        $totalLength = mb_strlen($string, $encoding);
        $startIndex = $index < -$totalLength ? 0 : $totalLength + $index;
    }

    $segmentLength = mb_strlen($segment, $encoding);
    $head = mb_substr($string, 0, $startIndex, $encoding);
    $tail = mb_substr($string, $startIndex + $segmentLength, null, $encoding);
    $fill = str_repeat(mb_substr($character, 0, 1, $encoding), $segmentLength);

    return $head.$fill.$tail;
}
