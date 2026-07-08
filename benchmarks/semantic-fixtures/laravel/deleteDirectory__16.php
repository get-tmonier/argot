# ID: src/Illuminate/Filesystem/Filesystem.php:739
<?php
public static function purgeDirectory($fs, $directory, $preserve = false)
{
    if (! $fs->isDirectory($directory)) {
        return false;
    }

    $items = new FilesystemIterator($directory);

    foreach ($items as $item) {
        // Recurse into real sub-directories; delete files and symlinks directly
        if ($item->isDir() && ! $item->isLink()) {
            static::purgeDirectory($fs, $item->getPathname());
        } else {
            $fs->delete($item->getPathname());
        }
    }

    unset($items);

    if (! $preserve) {
        @rmdir($directory);
    }

    return true;
}
