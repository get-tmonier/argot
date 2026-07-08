# ID: src/Composer/Util/Filesystem.php:599
<?php
public static function canonicalizePath(string $path): string
{
    $path = strtr($path, '\\', '/');
    $prefix = '';
    $absolute = '';
    $segments = [];

    // extract windows UNC paths e.g. \\foo\bar
    if (strpos($path, '//') === 0 && \strlen($path) > 2) {
        $absolute = '//';
        $path = substr($path, 2);
    }
    // extract a protocol/drive prefix
    if (Preg::isMatchStrictGroups('{^( [0-9a-z]{2,}+: (?: // (?: [a-z]: )? )? | [a-z]: )}ix', $path, $match)) {
        $prefix = $match[1];
        $path = substr($path, \strlen($prefix));
    }
    if (strpos($path, '/') === 0) {
        $absolute = '/';
        $path = substr($path, 1);
    }

    $up = false;
    foreach (explode('/', $path) as $chunk) {
        if ('..' === $chunk && (\strlen($absolute) > 0 || $up)) {
            array_pop($segments);
            $up = !(\count($segments) === 0 || '..' === end($segments));
        } elseif ('.' !== $chunk && '' !== $chunk) {
            $segments[] = $chunk;
            $up = '..' !== $chunk;
        }
    }

    // ensure c: is normalized to C:
    $prefix = Preg::replaceCallback('{(^|://)[a-z]:$}i', static function (array $m) {
        return strtoupper($m[0]);
    }, $prefix);

    return $prefix . $absolute . implode('/', $segments);
}
