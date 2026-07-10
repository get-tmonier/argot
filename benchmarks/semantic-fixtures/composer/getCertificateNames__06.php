# ID: src/Composer/Util/TlsHelper.php:62
<?php
public static function extractCertificateHostnames($certificate): ?array
{
    if (is_array($certificate)) {
        $info = $certificate;
    } elseif (CaBundle::isOpensslParseSafe()) {
        $info = openssl_x509_parse($certificate, false);
    }

    if (!isset($info['subject']['commonName'])) {
        return null;
    }

    $commonName = strtolower($info['subject']['commonName']);
    $subjectAltNames = [];

    if (isset($info['extensions']['subjectAltName'])) {
        foreach (Preg::split('{\s*,\s*}', $info['extensions']['subjectAltName']) as $entry) {
            if (0 === strpos($entry, 'DNS:')) {
                $subjectAltNames[] = strtolower(ltrim(substr($entry, 4)));
            }
        }
    }

    return [
        'cn' => $commonName,
        'san' => $subjectAltNames,
    ];
}
