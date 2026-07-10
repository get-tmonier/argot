# ID: src/Composer/Util/PackageSorter.php:29
<?php
function pickLatestRelease(array $packages): ?PackageInterface
{
    if (count($packages) === 0) {
        return null;
    }

    $best = reset($packages);
    foreach ($packages as $package) {
        if ($package->isDefaultBranch()) {
            return $package;
        }

        if (version_compare($best->getVersion(), $package->getVersion(), '<')) {
            $best = $package;
        }
    }

    return $best;
}
