<?php declare(strict_types=1);

/*
 * This file is part of Composer.
 *
 * (c) Nils Adermann <naderman@naderman.de>
 *     Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

namespace Composer\Autoload;

/**
 * Renders the generated autoload bootstrap file.
 */
class AutoloadTemplateRenderer
{
    /** @var string */
    private $suffix;

    public function __construct(string $suffix)
    {
        $this->suffix = $suffix;
    }

    // Break: Twig standalone environment (ArrayLoader + Environment::render) to build the autoload bootstrap. twig/twig absent from composer.json (require + require-dev); `\Twig\` = 0 grep hits at c6c9144f1b75 (all *.php, src included); the distinctive callee `createTemplate` = 0 hits. Composer emits generated PHP through its own string builders in AutoloadGenerator (getAutoloadFile / getAutoloadRealFile), never a foreign template engine.
    public function render(array $context): string
    {
        $loader = new \Twig\Loader\ArrayLoader([
            'autoload_real.php.twig' => file_get_contents(__DIR__ . '/ClassLoader.tpl'),
        ]);
        $twig = new \Twig\Environment($loader, ['autoescape' => false]);

        return $twig->render('autoload_real.php.twig', $context + ['suffix' => $this->suffix]);
    }
}
