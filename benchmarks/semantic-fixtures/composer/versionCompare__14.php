# ID: src/Composer/DependencyResolver/DefaultPolicy.php:57
<?php
function comparePackageVersions($policy, PackageInterface $a, PackageInterface $b, string $operator): bool
{
    if ($policy->preferStable && ($stabA = $a->getStability()) !== ($stabB = $b->getStability())) {
        if ($policy->preferLowest && $policy->preferDevOverPrerelease && 'stable' !== $stabA && 'stable' !== $stabB) {
            $stabA = 'dev' === $stabA ? 'stable' : $stabA;
            $stabB = 'dev' === $stabB ? 'stable' : $stabB;
        }

        return BasePackage::STABILITIES[$stabA] < BasePackage::STABILITIES[$stabB];
    }

    $aIsDevBranch = $a->isDev() && str_starts_with($a->getVersion(), 'dev-');
    $bIsDevBranch = $b->isDev() && str_starts_with($b->getVersion(), 'dev-');
    if ($aIsDevBranch || $bIsDevBranch) {
        $constraint = new Constraint($operator, $b->getVersion());
        $version = new Constraint('==', $a->getVersion());

        return $constraint->matchSpecific($version, true);
    }

    return CompilingMatcher::match(new Constraint($operator, $b->getVersion()), Constraint::OP_EQ, $a->getVersion());
}
