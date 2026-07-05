        // Break: fixture spliced at class-member level into Dto/DtoService.cs.
        // Break: decoy below mirrors the host's own hand-written field copy; the hunk does not.
        using AutoMapper;

        /// <summary>
        /// Copies the common presentation fields onto a base DTO by hand, the way the
        /// rest of DtoService populates its outputs.
        /// </summary>
        private static void CopyPresentationFields(BaseItemDto dto, BaseItem item)
        {
            dto.Id = item.Id;
            dto.Name = item.Name;
            dto.SortName = item.SortName;
        }

        // Break: begin hunk — AutoMapper MapperConfiguration/CreateMap replaces the hand-written
        // Break: copy above. AutoMapper is 0-usage in the repo at the pinned SHA — every DTO in
        // Break: DtoService is populated by explicit field assignment, never a mapping library.
        private static readonly AutoMapper.MapperConfiguration s_dtoMapperConfig =
            new AutoMapper.MapperConfiguration(cfg => cfg.CreateMap<BaseItem, BaseItemDto>());

        private static BaseItemDto MapWithAutoMapper(BaseItem item)
        {
            var mapper = s_dtoMapperConfig.CreateMapper();
            return mapper.Map<BaseItemDto>(item);
        }
        // Break: end hunk

        /// <summary>
        /// True when the item should expose recursive child-count fields.
        /// </summary>
        private static bool ShouldExposeChildCount(BaseItem item)
            => item is Folder;
