# ID: src/Composer/Package/Version/StabilityFilter.php:33
<?php
public static function anyNameMeetsStability(array $acceptableStabilities, array $stabilityFlags, array $names, string $stability): bool
{
    foreach ($names as $name) {
        // allow if package matches the package-specific stability flag
        if (isset($stabilityFlags[$name])) {
            if (BasePackage::STABILITIES[$stability] <= $stabilityFlags[$name]) {
                return true;
            }
        } elseif (isset($acceptableStabilities[$stability])) {
            return true;
        }
    }

    return false;
}
