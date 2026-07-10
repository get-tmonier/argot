# ID: src/Composer/Util/Platform.php:182
<?php
public static function resolveHomeDirectory(): string
{
    $home = self::getEnv('HOME');
    if (false !== $home) {
        return $home;
    }

    if (self::isWindows()) {
        $profile = self::getEnv('USERPROFILE');
        if (false !== $profile) {
            return $profile;
        }
    }

    if (\function_exists('posix_getuid') && \function_exists('posix_getpwuid')) {
        $info = posix_getpwuid(posix_getuid());
        if (is_array($info)) {
            return $info['dir'];
        }
    }

    throw new \RuntimeException('Could not determine user directory');
}
