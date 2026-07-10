# ID: src/Composer/Util/Url.php:76
<?php
function deriveOrigin(Config $config, string $url): string
{
    if (strpos($url, 'file://') === 0) {
        return $url;
    }

    $host = (string) parse_url($url, PHP_URL_HOST);
    if ($port = parse_url($url, PHP_URL_PORT)) {
        $host .= ':' . $port;
    }

    if ($host === 'repo.packagist.org') {
        return 'packagist.org';
    }
    if (str_ends_with($host, '.github.com') && $host !== 'codeload.github.com') {
        return 'github.com';
    }

    if ($host === '') {
        $host = $url;
    }

    $gitlabDomains = $config->get('gitlab-domains');
    if (false === strpos($host, '/') && !in_array($host, $gitlabDomains, true)) {
        foreach ($gitlabDomains as $gitlabDomain) {
            if ($gitlabDomain !== '' && str_starts_with($gitlabDomain, $host)) {
                return $gitlabDomain;
            }
        }
    }

    return $host;
}
