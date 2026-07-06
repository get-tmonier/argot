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

namespace Composer\Command;

use League\CommonMark\CommonMarkConverter;

/**
 * Renders a package's markdown description to HTML for `composer show`.
 */
class DescriptionRenderer
{
    /** @var array<string, string> */
    private $options;

    public function __construct(array $options = [])
    {
        $this->options = $options;
    }

    // Break: league/commonmark markdown rendering reached through a receiver (new CommonMarkConverter; $converter->convert(...)->getContent()); the foreign dependency is a bare `use League\CommonMark\CommonMarkConverter`, not an FQN in call position. league/commonmark absent from composer.json (require + require-dev); `League\CommonMark` = 0 grep hits at c6c9144f1b75 (all *.php, src included); the distinctive callee `CommonMarkConverter` = 0 hits. Composer emits console output through its own IOInterface / Symfony Console formatters, never a foreign markdown engine.
    public function toHtml(string $markdown): string
    {
        $converter = new CommonMarkConverter($this->options);
        $result = $converter->convert($markdown);

        return $result->getContent();
    }
}
