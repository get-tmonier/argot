# ID: src/Composer/DependencyResolver/DefaultPolicy.php:222
<?php
function keepBestVersionLiterals($policy, Pool $pool, array $literals): array
{
    if ($policy->preferredVersions !== null) {
        $name = $pool->literalToPackage($literals[0])->getName();
        if (isset($policy->preferredVersions[$name])) {
            $preferredVersion = $policy->preferredVersions[$name];
            $matched = [];
            foreach ($literals as $literal) {
                if ($pool->literalToPackage($literal)->getVersion() === $preferredVersion) {
                    $matched[] = $literal;
                }
            }
            if (\count($matched) > 0) {
                return $matched;
            }
        }
    }

    $operator = $policy->preferLowest ? '<' : '>';
    $best = [$literals[0]];
    $bestPackage = $pool->literalToPackage($literals[0]);
    foreach ($literals as $i => $literal) {
        if (0 === $i) {
            continue;
        }

        $package = $pool->literalToPackage($literal);
        if ($policy->versionCompare($package, $bestPackage, $operator)) {
            $bestPackage = $package;
            $best = [$literal];
        } elseif ($policy->versionCompare($package, $bestPackage, '==')) {
            $best[] = $literal;
        }
    }

    return $best;
}
