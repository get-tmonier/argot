# ID: src/Composer/Autoload/ClassMapGenerator.php:63
<?php
public static function buildClassMap($path, ?string $excluded = null, ?IOInterface $io = null, ?string $namespace = null, ?string $autoloadType = null, array &$scannedFiles = []): array
{
    $generator = new \Composer\ClassMapGenerator\ClassMapGenerator(['php', 'inc', 'hh']);
    $fileList = new FileList();
    $fileList->files = $scannedFiles;
    $generator->avoidDuplicateScans($fileList);
    $generator->scanPaths($path, $excluded, $autoloadType ?? 'classmap', $namespace);

    $classMap = $generator->getClassMap();
    $scannedFiles = $fileList->files;

    if ($io !== null) {
        foreach ($classMap->getPsrViolations() as $msg) {
            $io->writeError("<warning>$msg</warning>");
        }

        foreach ($classMap->getAmbiguousClasses() as $class => $paths) {
            if (count($paths) > 1) {
                $io->writeError('<warning>Warning: Ambiguous class resolution, "' . $class . '" was found ' . (count($paths) + 1) . 'x: in "' . $classMap->getClassPath($class) . '" and "' . implode('", "', $paths) . '", the first will be used.</warning>');
            } else {
                $io->writeError('<warning>Warning: Ambiguous class resolution, "' . $class . '" was found in both "' . $classMap->getClassPath($class) . '" and "' . implode('", "', $paths) . '", the first will be used.</warning>');
            }
        }
    }

    return $classMap->getMap();
}
