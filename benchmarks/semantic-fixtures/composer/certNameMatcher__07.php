# ID: src/Composer/Util/TlsHelper.php:172
<?php
private static function buildCertNameMatcher(string $certName): ?callable
{
    $wildcards = substr_count($certName, '*');

    if (0 === $wildcards) {
        return static function ($hostname) use ($certName): bool {
            return $hostname === $certName;
        };
    }

    if (1 !== $wildcards) {
        return null;
    }

    $components = explode('.', $certName);
    if (count($components) < 3) {
        return null;
    }

    $firstComponent = $components[0];
    if ('*' !== $firstComponent[strlen($firstComponent) - 1]) {
        return null;
    }

    $wildcardRegex = preg_quote($certName);
    $wildcardRegex = str_replace('\\*', '[a-z0-9-]+', $wildcardRegex);
    $wildcardRegex = "{^{$wildcardRegex}$}";

    return static function ($hostname) use ($wildcardRegex): bool {
        return Preg::isMatch($wildcardRegex, $hostname);
    };
}
