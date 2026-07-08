# ID: src/Composer/DependencyResolver/Transaction.php:230
<?php
function findRootPackages($transaction): array
{
    $roots = $transaction->resultPackageMap;

    foreach ($transaction->resultPackageMap as $packageHash => $package) {
        if (!isset($roots[$packageHash])) {
            continue;
        }

        foreach ($package->getRequires() as $link) {
            foreach ($transaction->getProvidersInResult($link) as $provider) {
                if ($provider !== $package) {
                    unset($roots[spl_object_hash($provider)]);
                }
            }
        }
    }

    return $roots;
}
