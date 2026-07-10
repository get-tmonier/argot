# ID: src/Composer/Package/Version/VersionParser.php:50
<?php
public static function splitNameVersionPairs(array $pairs): array
{
    $pairs = array_values($pairs);
    $count = count($pairs);
    $result = [];

    for ($i = 0; $i < $count; $i++) {
        $pair = Preg::replace('{^([^=: ]+)[=: ](.*)$}', '$1 $2', trim($pairs[$i]));

        $next = $pairs[$i + 1] ?? null;
        if (false === strpos($pair, ' ') && $next !== null && false === strpos($next, '/') && !Preg::isMatch('{(?<=[a-z0-9_/-])\*|\*(?=[a-z0-9_/-])}i', $next) && !PlatformRepository::isPlatformPackage($next)) {
            $pair .= ' ' . $next;
            $i++;
        }

        if (strpos($pair, ' ')) {
            [$name, $version] = explode(' ', $pair, 2);
            $result[] = ['name' => $name, 'version' => $version];
        } else {
            $result[] = ['name' => $pair];
        }
    }

    return $result;
}
