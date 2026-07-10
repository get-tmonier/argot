# ID: src/Illuminate/Support/Str.php:1565
<?php
public static function urlSlug($title, $separator = '-', $language = 'en', $dictionary = ['@' => 'at'])
{
    // Transliterate to ASCII when a language is provided
    $text = $language ? static::ascii($title, $language) : $title;

    // Normalize the "other" delimiter into the requested separator
    $opposite = $separator === '-' ? '_' : '-';
    $text = preg_replace('!['.preg_quote($opposite).']+!u', $separator, $text);

    // Expand dictionary words, padded on both sides by the separator
    foreach ($dictionary as $word => $replacement) {
        $dictionary[$word] = $separator.$replacement.$separator;
    }
    $text = str_replace(array_keys($dictionary), array_values($dictionary), $text);

    // Lower-case, then strip anything that is not letter/number/separator/space
    $text = preg_replace('![^'.preg_quote($separator).'\pL\pN\s]+!u', '', static::lower($text));

    // Collapse runs of separators and whitespace down to a single separator
    $text = preg_replace('!['.preg_quote($separator).'\s]+!u', $separator, $text);

    return trim($text, $separator);
}
