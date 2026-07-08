# ID: src/Illuminate/Support/Str.php:444
<?php
public static function snippet($text, $phrase = '', $options = [])
{
    $radius = $options['radius'] ?? 100;
    $omission = $options['omission'] ?? '...';

    preg_match('/^(.*?)('.preg_quote((string) $phrase, '/').')(.*)$/iu', (string) $text, $matches);

    if (empty($matches)) {
        return null;
    }

    $leading = ltrim($matches[1]);
    $leading = Str::of(mb_substr($leading, max(mb_strlen($leading, 'UTF-8') - $radius, 0), $radius, 'UTF-8'))->ltrim()->unless(
        fn ($clipped) => $clipped->exactly($leading),
        fn ($clipped) => $clipped->prepend($omission),
    );

    $trailing = rtrim($matches[3]);
    $trailing = Str::of(mb_substr($trailing, 0, $radius, 'UTF-8'))->rtrim()->unless(
        fn ($clipped) => $clipped->exactly($trailing),
        fn ($clipped) => $clipped->append($omission),
    );

    return $leading->append($matches[2], $trailing)->toString();
}
