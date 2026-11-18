<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * One part of a package, as the engine's `tree` shows it.
 */
final readonly class TreePart
{
    /**
     * @param string $name the part's name (`/xl/workbook.xml`)
     * @param string|null $contentType the part's content type, null when none is declared
     * @param bool $contentTypeKnown whether the content type could be determined
     * @param int $size the part's size in bytes, uncompressed
     * @param int $compressedSize the part's size in the archive
     * @param bool $readable whether the part could be read
     * @param bool|null $referenced whether a relationship points at the part, null when that is not known
     * @param bool $relationshipsKnown whether the part's relationships could be read
     * @param list<TreeRelationship> $relationships the relationships the part holds
     */
    public function __construct(
        public string $name,
        public ?string $contentType,
        public bool $contentTypeKnown,
        public int $size,
        public int $compressedSize,
        public bool $readable,
        public ?bool $referenced,
        public bool $relationshipsKnown,
        public array $relationships,
    ) {}
}
