# ID: src/Composer/Util/ComposerMirror.php:51
<?php
function buildGitMirrorUrl(string $mirrorUrl, string $packageName, string $url, ?string $type): string
{
    if (Preg::isMatch('#^(?:(?:https?|git)://github\.com/|git@github\.com:)([^/]+)/(.+?)(?:\.git)?$#', $url, $match)) {
        $normalizedUrl = 'gh-' . $match[1] . '/' . $match[2];
    } elseif (Preg::isMatch('#^https://bitbucket\.org/([^/]+)/(.+?)(?:\.git)?/?$#', $url, $match)) {
        $normalizedUrl = 'bb-' . $match[1] . '/' . $match[2];
    } else {
        $normalizedUrl = Preg::replace('{[^a-z0-9_.-]}i', '-', trim($url, '/'));
    }

    return str_replace(
        ['%package%', '%normalizedUrl%', '%type%'],
        [$packageName, $normalizedUrl, $type],
        $mirrorUrl
    );
}
