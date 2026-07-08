# ID: src/Composer/Repository/RepositoryUtils.php:36
<?php
public static function collectRequiredPackages(array $packages, PackageInterface $requirer, bool $includeRequireDev = false, array $bucket = []): array
{
    $requires = $requirer->getRequires();
    if ($includeRequireDev) {
        $requires = array_merge($requires, $requirer->getDevRequires());
    }

    foreach ($packages as $candidate) {
        foreach ($candidate->getNames() as $name) {
            if (!isset($requires[$name])) {
                continue;
            }
            if (!in_array($candidate, $bucket, true)) {
                $bucket[] = $candidate;
                $bucket = self::collectRequiredPackages($packages, $candidate, false, $bucket);
            }
            break;
        }
    }

    return $bucket;
}
