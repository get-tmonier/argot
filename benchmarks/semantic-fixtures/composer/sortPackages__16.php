# ID: src/Composer/Json/JsonManipulator.php:126
<?php
public static function orderRequirements(array &$packages = []): void
{
    $prefix = static function ($requirement): string {
        if (!PlatformRepository::isPlatformPackage($requirement)) {
            return '5-' . $requirement;
        }

        return Preg::replace(
            ['/^php/', '/^hhvm/', '/^ext/', '/^lib/', '/^\D/'],
            ['0-$0', '1-$0', '2-$0', '3-$0', '4-$0'],
            $requirement
        );
    };

    uksort($packages, static function ($a, $b) use ($prefix): int {
        return strnatcmp($prefix($a), $prefix($b));
    });
}
