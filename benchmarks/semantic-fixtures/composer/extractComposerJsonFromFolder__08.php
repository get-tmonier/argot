# ID: src/Composer/Util/Tar.php:37
<?php
private static function readManifestFromArchive(\PharData $phar): string
{
    if (isset($phar['composer.json'])) {
        return $phar['composer.json']->getContent();
    }

    $topLevelPaths = [];
    foreach ($phar as $entry) {
        if (!$entry->isDir()) {
            continue;
        }
        $topLevelPaths[$entry->getBasename()] = true;
        if (\count($topLevelPaths) > 1) {
            throw new \RuntimeException('Archive has more than one top level directories, and no composer.json was found on the top level, so it\'s an invalid archive. Top level paths found were: ' . implode(',', array_keys($topLevelPaths)));
        }
    }

    $composerJsonPath = key($topLevelPaths) . '/composer.json';
    if (\count($topLevelPaths) > 0 && isset($phar[$composerJsonPath])) {
        return $phar[$composerJsonPath]->getContent();
    }

    throw new \RuntimeException('No composer.json found either at the top level or within the topmost directory');
}
