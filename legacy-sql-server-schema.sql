
CREATE TABLE [andrew].[Board] (
    [Id]       INT              IDENTITY (1, 1) NOT NULL,
    [BoardId]  UNIQUEIDENTIFIER NOT NULL,
    [UserId]   UNIQUEIDENTIFIER NOT NULL,
    [Title]    NVARCHAR (250)   NOT NULL,
    [Content]  NVARCHAR (MAX)   DEFAULT ('{}') NOT NULL,
    [Created]  DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified] DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]  BIT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [dbo].[SpeareDocs] (
    [Id]               INT              IDENTITY (1, 1) NOT NULL,
    [DocId]            UNIQUEIDENTIFIER DEFAULT (newid()) NOT NULL,
    [UserId]           UNIQUEIDENTIFIER NOT NULL,
    [Title]            NVARCHAR (150)   DEFAULT ('?') NOT NULL,
    [Subtitle]         NVARCHAR (500)   DEFAULT ('?') NOT NULL,
    [Document]         NVARCHAR (MAX)   DEFAULT ('?') NOT NULL,
    [DocType]          INT              DEFAULT ((1)) NOT NULL,
    [Access]           INT              DEFAULT ((1)) NOT NULL,
    [Created]          DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified]         DATETIME         DEFAULT (getdate()) NOT NULL,
    [Published]        INT              DEFAULT ((0)) NOT NULL,
    [Search]           NVARCHAR (MAX)   DEFAULT ('?') NOT NULL,
    [Updated]          DATETIME         DEFAULT ('1/1/2016') NOT NULL,
    [Deleted]          INT              DEFAULT ((0)) NOT NULL,
    [RegCode]          NCHAR (8)        DEFAULT (left(newid(),(8))) NOT NULL,
    [OrderList]        NVARCHAR (MAX)   NULL,
    [Revisions]        NVARCHAR (MAX)   NULL,
    [Pinned]           INT              DEFAULT ((0)) NULL,
    [ConnectionsCount] INT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [dbo].[Speare_Blocks] (
    [Id]       INT              IDENTITY (1, 1) NOT NULL,
    [UserId]   UNIQUEIDENTIFIER NOT NULL,
    [DocId]    UNIQUEIDENTIFIER NOT NULL,
    [BlockId]  NVARCHAR (40)    DEFAULT (newid()) NOT NULL,
    [Content]  NVARCHAR (MAX)   DEFAULT ('') NOT NULL,
    [Tags]     NVARCHAR (MAX)   DEFAULT ('') NOT NULL,
    [NumOrder] INT              DEFAULT ((1)) NOT NULL,
    [Created]  DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified] DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]  INT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [dbo].[SpeareWorkspaces] (
    [Id]          INT              IDENTITY (1, 1) NOT NULL,
    [WorkspaceId] UNIQUEIDENTIFIER DEFAULT (newid()) NOT NULL,
    [UserId]      UNIQUEIDENTIFIER NOT NULL,
    [Title]       NVARCHAR (150)   DEFAULT ('?') NOT NULL,
    [Workstate]   NVARCHAR (MAX)   DEFAULT ('?') NOT NULL,
    [Pinned]      INT              DEFAULT ((0)) NOT NULL,
    [Created]     DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified]    DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]     INT              DEFAULT ((0)) NOT NULL,
    [RegCode]     NCHAR (8)        DEFAULT (left(newid(),(8))) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [dbo].[Speare_WorkspaceTrees] (
    [Id]            INT              IDENTITY (1, 1) NOT NULL,
    [UserId]        UNIQUEIDENTIFIER NOT NULL,
    [NodeId]        UNIQUEIDENTIFIER NOT NULL,
    [SpaceId]       UNIQUEIDENTIFIER NOT NULL,
    [ParentNodeId]  UNIQUEIDENTIFIER DEFAULT (CONVERT([uniqueidentifier],CONVERT([binary],(0)))) NOT NULL,
    [NodeTitle]     NVARCHAR (250)   DEFAULT ('Untitled') NOT NULL,
    [NodeOrder]     INT              DEFAULT ((1)) NOT NULL,
    [NodeCollapsed] BIT              DEFAULT ((0)) NOT NULL,
    [NodeHidden]    BIT              DEFAULT ((0)) NOT NULL,
    [NodeType]      VARCHAR (20)     DEFAULT ('board') NOT NULL,
    [BoardId]       UNIQUEIDENTIFIER DEFAULT (CONVERT([uniqueidentifier],CONVERT([binary],(0)))) NOT NULL,
    [DocumentId]    UNIQUEIDENTIFIER DEFAULT (CONVERT([uniqueidentifier],CONVERT([binary],(0)))) NOT NULL,
    [Deleted]       INT              DEFAULT ((0)) NOT NULL,
    [Created]       DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified]      DATETIME         DEFAULT (getdate()) NOT NULL,
    [SecretKey]     VARCHAR (40)     DEFAULT (CONVERT([varchar](100),hashbytes('SHA2_256',CONVERT([binary],newid())))) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [dbo].[SpeareUserSettings] (
    [Id]       INT            IDENTITY (1, 1) NOT NULL,
    [UserId]   NVARCHAR (40)  NOT NULL,
    [Settings] NVARCHAR (MAX) NULL,
    [Created]  DATETIME       DEFAULT (getdate()) NOT NULL,
    [Modified] DATETIME       DEFAULT (getdate()) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[CardSettings] (
    [Id]                  INT              IDENTITY (1, 1) NOT NULL,
    [CardId]              UNIQUEIDENTIFIER NOT NULL,
    [UserId]              UNIQUEIDENTIFIER NOT NULL,
    [CardSettings]        NVARCHAR (MAX)   DEFAULT ('{}') NOT NULL,
    [CardSettingsVersion] VARCHAR (5)      DEFAULT ('1.0') NOT NULL,
    [Created]             DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified]            DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]             BIT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[CardSnapshot] (
    [Id]           INT              IDENTITY (1, 1) NOT NULL,
    [UserId]       UNIQUEIDENTIFIER NOT NULL,
    [SnapshotId]   UNIQUEIDENTIFIER NOT NULL,
    [SnapshotName] NVARCHAR (80)    DEFAULT ('') NOT NULL,
    [CardId]       UNIQUEIDENTIFIER NOT NULL,
    [Title]        NVARCHAR (250)   NOT NULL,
    [Content]      NVARCHAR (MAX)   DEFAULT ('[]') NOT NULL,
    [Created]      DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]      BIT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[Card] (
    [Id]          INT              IDENTITY (1, 1) NOT NULL,
    [CardId]      UNIQUEIDENTIFIER NOT NULL,
    [Title]       NVARCHAR (250)   NOT NULL,
    [Content]     NVARCHAR (MAX)   DEFAULT ('[]') NOT NULL,
    [Created]     DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified]    DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]     BIT              DEFAULT ((0)) NOT NULL,
    [UserId]      UNIQUEIDENTIFIER NOT NULL,
    [DocId]       UNIQUEIDENTIFIER DEFAULT (CONVERT([uniqueidentifier],CONVERT([binary],(0)))) NOT NULL,
    [CardVersion] VARCHAR (5)      DEFAULT ('1.0') NOT NULL,
    [WordCount]   INT              DEFAULT ((-1)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[DataTable] (
    [Id]       INT              IDENTITY (1, 1) NOT NULL,
    [TableId]  UNIQUEIDENTIFIER NOT NULL,
    [UserId]   UNIQUEIDENTIFIER NOT NULL,
    [Title]    NVARCHAR (250)   NOT NULL,
    [TableDef] NVARCHAR (MAX)   DEFAULT ('{}') NOT NULL,
    [Created]  DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified] DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]  BIT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[DeletedResource] (
    [Id]           INT              IDENTITY (1, 1) NOT NULL,
    [UserId]       UNIQUEIDENTIFIER NOT NULL,
    [ResourceId]   UNIQUEIDENTIFIER NOT NULL,
    [ResourceType] VARCHAR (40)     NOT NULL,
    [DeleteDate]   DATETIME         DEFAULT (getdate()) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[DeletedUser] (
    [Id]                        INT              IDENTITY (1, 1) NOT NULL,
    [DeletedUserId]             UNIQUEIDENTIFIER NOT NULL,
    [PaymentProcessorAccountId] VARCHAR (40)     NOT NULL,
    [DeleteNotes]               NVARCHAR (MAX)   NULL,
    [Completed]                 BIT              DEFAULT ((0)) NOT NULL,
    [DeletedDate]               DATETIME         DEFAULT (getdate()) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[DocCardSnapshot] (
    [Id]                INT              IDENTITY (1, 1) NOT NULL,
    [UserId]            UNIQUEIDENTIFIER NOT NULL,
    [DocCardSnapshotId] UNIQUEIDENTIFIER NOT NULL,
    [CardSnapshotId]    UNIQUEIDENTIFIER NOT NULL,
    [DocSnapshotId]     UNIQUEIDENTIFIER NOT NULL,
    [Deleted]           BIT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[DocSettings] (
    [Id]                 INT              IDENTITY (1, 1) NOT NULL,
    [DocId]              UNIQUEIDENTIFIER NOT NULL,
    [UserId]             UNIQUEIDENTIFIER NOT NULL,
    [DocSettings]        NVARCHAR (MAX)   DEFAULT ('{}') NOT NULL,
    [DocSettingsVersion] VARCHAR (5)      DEFAULT ('1.0') NOT NULL,
    [Created]            DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified]           DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]            BIT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[DocSnapshot] (
    [Id]           INT              IDENTITY (1, 1) NOT NULL,
    [UserId]       UNIQUEIDENTIFIER NOT NULL,
    [SnapshotId]   UNIQUEIDENTIFIER NOT NULL,
    [SnapshotName] NVARCHAR (80)    NOT NULL,
    [DocId]        UNIQUEIDENTIFIER NOT NULL,
    [Title]        NVARCHAR (250)   NOT NULL,
    [Content]      NVARCHAR (MAX)   DEFAULT ('[]') NOT NULL,
    [Created]      DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]      BIT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO


CREATE TABLE [andrew].[FavoritesList] (
    [Id]          INT              IDENTITY (1, 1) NOT NULL,
    [FavoritesId] UNIQUEIDENTIFIER NOT NULL,
    [UserId]      UNIQUEIDENTIFIER NOT NULL,
    [Content]     NVARCHAR (MAX)   DEFAULT ('{}') NOT NULL,
    [Created]     DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified]    DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]     BIT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[Folder] (
    [Id]         INT              IDENTITY (1, 1) NOT NULL,
    [UserId]     UNIQUEIDENTIFIER NOT NULL,
    [FolderId]   UNIQUEIDENTIFIER NOT NULL,
    [Title]      NVARCHAR (250)   NOT NULL,
    [SourceType] VARCHAR (20)     NOT NULL,
    [SourceId]   UNIQUEIDENTIFIER NOT NULL,
    [Created]    DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified]   DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]    BIT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO


CREATE TABLE [andrew].[Image] (
    [Id]        INT              IDENTITY (1, 1) NOT NULL,
    [UserId]    UNIQUEIDENTIFIER NOT NULL,
    [Container] VARCHAR (40)     NOT NULL,
    [BlobName]  VARCHAR (100)    NOT NULL,
    [ImageType] VARCHAR (20)     NOT NULL,
    [ImageSize] INT              DEFAULT ((0)) NOT NULL,
    [Created]   DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]   BIT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[InternalLink] (
    [Id]       INT              IDENTITY (1, 1) NOT NULL,
    [LinkId]   UNIQUEIDENTIFIER NOT NULL,
    [FromId]   UNIQUEIDENTIFIER NOT NULL,
    [ToId]     UNIQUEIDENTIFIER NOT NULL,
    [ToType]   VARCHAR (40)     NOT NULL,
    [Created]  DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified] DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]  BIT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO


CREATE TABLE [andrew].[MissingResource] (
    [Id]           INT              IDENTITY (1, 1) NOT NULL,
    [UserId]       UNIQUEIDENTIFIER NOT NULL,
    [ResourceId]   UNIQUEIDENTIFIER NOT NULL,
    [ResourceType] VARCHAR (40)     NOT NULL,
    [Created]      DATETIME         DEFAULT (getdate()) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[NavHistory] (
    [Id]       INT              IDENTITY (1, 1) NOT NULL,
    [UserId]   UNIQUEIDENTIFIER NOT NULL,
    [Content]  NVARCHAR (MAX)   DEFAULT ('{}') NOT NULL,
    [Created]  DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified] DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]  BIT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[OmniSearch] (
    [Id]            INT              IDENTITY (1, 1) NOT NULL,
    [UserId]        UNIQUEIDENTIFIER NOT NULL,
    [ItemId]        UNIQUEIDENTIFIER NOT NULL,
    [ItemType]      VARCHAR (40)     NOT NULL,
    [Content]       NVARCHAR (MAX)   DEFAULT ('') NOT NULL,
    [ResourceId]    UNIQUEIDENTIFIER DEFAULT (CONVERT([uniqueidentifier],CONVERT([binary],(0)))) NOT NULL,
    [ResourceTitle] NVARCHAR (250)   DEFAULT ('') NOT NULL,
    [DocId]         UNIQUEIDENTIFIER DEFAULT (CONVERT([uniqueidentifier],CONVERT([binary],(0)))) NOT NULL,
    [Created]       DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified]      DATETIME         DEFAULT (getdate()) NOT NULL,
    [Favorite]      BIT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[PageLink] (
    [Id]         INT              IDENTITY (1, 1) NOT NULL,
    [UserId]     UNIQUEIDENTIFIER NOT NULL,
    [LinkId]     UNIQUEIDENTIFIER NOT NULL,
    [FromPageId] UNIQUEIDENTIFIER NOT NULL,
    [ToId]       UNIQUEIDENTIFIER NOT NULL,
    [Created]    DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified]   DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]    BIT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[PinnedResources] (
    [Id]                INT              IDENTITY (1, 1) NOT NULL,
    [PinnedResourcesId] UNIQUEIDENTIFIER NOT NULL,
    [UserId]            UNIQUEIDENTIFIER NOT NULL,
    [Content]           NVARCHAR (MAX)   DEFAULT ('{}') NOT NULL,
    [Created]           DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified]          DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]           BIT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO



CREATE TABLE [andrew].[PublishedDocLink] (
    [Id]            INT              IDENTITY (1, 1) NOT NULL,
    [UserId]        UNIQUEIDENTIFIER NOT NULL,
    [LinkId]        UNIQUEIDENTIFIER NOT NULL,
    [DocId]         UNIQUEIDENTIFIER NOT NULL,
    [DocSnapshotId] UNIQUEIDENTIFIER NOT NULL,
    [Created]       DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified]      DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]       BIT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[ShareLink] (
    [Id]           INT              IDENTITY (1, 1) NOT NULL,
    [UserId]       UNIQUEIDENTIFIER NOT NULL,
    [ShareLinkId]  VARCHAR (40)     NOT NULL,
    [ResourceId]   UNIQUEIDENTIFIER NOT NULL,
    [ResourceType] VARCHAR (40)     NOT NULL,
    [Created]      DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]      BIT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[SpeareUser] (
    [Id]         INT              IDENTITY (1, 1) NOT NULL,
    [UserId]     UNIQUEIDENTIFIER DEFAULT (newid()) NOT NULL,
    [Email]      NVARCHAR (250)   NOT NULL,
    [FirstName]  NVARCHAR (50)    DEFAULT ('?') NOT NULL,
    [LastName]   NVARCHAR (50)    DEFAULT (left(newid(),(4))) NOT NULL,
    [Created]    DATETIME         DEFAULT (getdate()) NOT NULL,
    [LastLogin]  DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified]   DATETIME         DEFAULT (getdate()) NOT NULL,
    [ProfilePic] NVARCHAR (500)   DEFAULT ('?') NOT NULL,
    [Deleted]    BIT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[Stack] (
    [Id]       INT              IDENTITY (1, 1) NOT NULL,
    [StackId]  UNIQUEIDENTIFIER NOT NULL,
    [Title]    NVARCHAR (250)   NOT NULL,
    [Content]  NVARCHAR (MAX)   DEFAULT ('{}') NOT NULL,
    [Created]  DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified] DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]  BIT              DEFAULT ((0)) NOT NULL,
    [UserId]   UNIQUEIDENTIFIER NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[TeamFeatureEarlyAccess] (
    [Id]      INT              IDENTITY (1, 1) NOT NULL,
    [UserId]  UNIQUEIDENTIFIER NOT NULL,
    [Created] DATETIME         DEFAULT (getdate()) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[TeamMember] (
    [Id]           INT              IDENTITY (1, 1) NOT NULL,
    [MemberId]     UNIQUEIDENTIFIER DEFAULT (newid()) NOT NULL,
    [TeamId]       UNIQUEIDENTIFIER NOT NULL,
    [MemberUserId] UNIQUEIDENTIFIER NOT NULL,
    [Created]      DATETIME         DEFAULT (getdate()) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[TeamResource] (
    [Id]            INT              IDENTITY (1, 1) NOT NULL,
    [ResourceId]    UNIQUEIDENTIFIER NOT NULL,
    [ResourceType]  VARCHAR (40)     NULL,
    [OwnerMemberId] UNIQUEIDENTIFIER NOT NULL,
    [TeamId]        UNIQUEIDENTIFIER NOT NULL,
    [Created]       DATETIME         DEFAULT (getdate()) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[Team] (
    [Id]       INT              IDENTITY (1, 1) NOT NULL,
    [UserId]   UNIQUEIDENTIFIER NOT NULL,
    [TeamId]   UNIQUEIDENTIFIER NOT NULL,
    [Title]    NVARCHAR (250)   DEFAULT ('') NOT NULL,
    [Created]  DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified] DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]  BIT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO


CREATE TABLE [andrew].[UserSettings] (
    [Id]       INT              IDENTITY (1, 1) NOT NULL,
    [UserId]   UNIQUEIDENTIFIER NOT NULL,
    [Settings] NVARCHAR (MAX)   DEFAULT ('{}') NOT NULL,
    [Created]  DATETIME2 (3)    DEFAULT (sysdatetime()) NOT NULL,
    [Modified] DATETIME2 (3)    DEFAULT (sysdatetime()) NOT NULL,
    [Deleted]  BIT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[UserToken] (
    [Id]      INT              IDENTITY (1, 1) NOT NULL,
    [UserId]  UNIQUEIDENTIFIER NOT NULL,
    [TokenId] UNIQUEIDENTIFIER NOT NULL,
    [Created] DATETIME         DEFAULT (getdate()) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[V4LegacyDocLink] (
    [Id]          INT              IDENTITY (1, 1) NOT NULL,
    [LinkId]      UNIQUEIDENTIFIER NOT NULL,
    [WorkspaceId] UNIQUEIDENTIFIER NOT NULL,
    [UserId]      UNIQUEIDENTIFIER NOT NULL,
    [Title]       NVARCHAR (250)   NOT NULL,
    [Content]     NVARCHAR (MAX)   DEFAULT ('') NOT NULL,
    [Created]     DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified]    DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]     BIT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[V4LegacyDocLinkVisits] (
    [Id]     INT              IDENTITY (1, 1) NOT NULL,
    [LinkId] UNIQUEIDENTIFIER NOT NULL,
    [Visits] INT              DEFAULT ((0)) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[WorkspaceDetails] (
    [Id]          INT              IDENTITY (1, 1) NOT NULL,
    [WorkspaceId] UNIQUEIDENTIFIER NOT NULL,
    [Content]     NVARCHAR (MAX)   DEFAULT ('{}') NOT NULL,
    [Modified]    DATETIME         DEFAULT (getdate()) NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO


CREATE TABLE [andrew].[WorkspaceSettings] (
    [Id]          INT              IDENTITY (1, 1) NOT NULL,
    [WorkspaceId] UNIQUEIDENTIFIER NOT NULL,
    [Settings]    NVARCHAR (MAX)   DEFAULT ('{}') NOT NULL,
    [Created]     DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified]    DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]     BIT              DEFAULT ((0)) NOT NULL,
    [UserId]      UNIQUEIDENTIFIER NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO

CREATE TABLE [andrew].[Workspace] (
    [Id]               INT              IDENTITY (1, 1) NOT NULL,
    [WorkspaceId]      UNIQUEIDENTIFIER NOT NULL,
    [Title]            NVARCHAR (250)   NOT NULL,
    [Content]          NVARCHAR (MAX)   DEFAULT ('[]') NOT NULL,
    [Created]          DATETIME         DEFAULT (getdate()) NOT NULL,
    [Modified]         DATETIME         DEFAULT (getdate()) NOT NULL,
    [Deleted]          BIT              DEFAULT ((0)) NOT NULL,
    [UserId]           UNIQUEIDENTIFIER NOT NULL,
    [WorkspaceVersion] VARCHAR (5)      CONSTRAINT [DEFAULT_Workspace_WorkspaceVersion] DEFAULT ('1.0') NOT NULL,
    PRIMARY KEY CLUSTERED ([Id] ASC)
);


GO
