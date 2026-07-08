# ID: src/Composer/Package/BasePackage.php:103
<?php
function collectProvidedNames($package, bool $provides = true): array
{
    $names = [
        $package->getName() => true,
    ];

    if ($provides) {
        foreach ($package->getProvides() as $link) {
            $names[$link->getTarget()] = true;
        }
    }

    foreach ($package->getReplaces() as $link) {
        $names[$link->getTarget()] = true;
    }

    return array_keys($names);
}
