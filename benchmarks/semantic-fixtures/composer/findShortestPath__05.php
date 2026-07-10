# ID: src/Composer/Util/Filesystem.php:466
<?php
public static function computeRelativePath($fs, string $from, string $to, bool $directories = false, bool $preferRelative = false)
{
    if (!$fs->isAbsolutePath($from) || !$fs->isAbsolutePath($to)) {
        throw new \InvalidArgumentException(sprintf('$from (%s) and $to (%s) must be absolute paths.', $from, $to));
    }

    $from = $fs->normalizePath($from);
    $to = $fs->normalizePath($to);

    if ($directories) {
        $from = rtrim($from, '/') . '/dummy_file';
    }
    if (\dirname($from) === \dirname($to)) {
        return './' . basename($to);
    }

    $commonPath = $to;
    while (strpos($from . '/', $commonPath . '/') !== 0 && '/' !== $commonPath && !Preg::isMatch('{^[A-Z]:/?$}i', $commonPath)) {
        $commonPath = strtr(\dirname($commonPath), '\\', '/');
    }

    if (0 !== strpos($from, $commonPath)) {
        return $to;
    }

    $commonPath = rtrim($commonPath, '/') . '/';
    $depth = substr_count((string) substr($from, \strlen($commonPath)), '/');

    if (!$preferRelative && '/' === $commonPath && $depth > 1) {
        return $to;
    }

    $result = str_repeat('../', $depth) . substr($to, \strlen($commonPath));

    return \strlen($result) === 0 ? './' : $result;
}
