# Libiamo 设计文档 v1

## 1. SQL Schema

### 1.1 alpha 阶段

#### 1.1.1 `users` — 用户表

```postgresql
CREATE TYPE user_role AS ENUM ('learner', 'admin');
CREATE TYPE token_purpose AS ENUM ('email_verification', 'password_reset');

CREATE TYPE language_code AS ENUM ('en', 'es', 'fr'); -- BCP 47
CREATE DOMAIN native_language_code AS VARCHAR(5)
CHECK (
    VALUE ~ '^[a-z]{2}(-[A-Z]{2})?$'
);

-- Users Table
CREATE TABLE users (
    id              INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    email           VARCHAR(128) UNIQUE NOT NULL,
    password_hash   TEXT NOT NULL,

    is_verified     BOOLEAN NOT NULL DEFAULT FALSE,
    role            user_role NOT NULL DEFAULT 'learner',
    timezone        VARCHAR(64) NOT NULL DEFAULT 'UTC',
  	nickname        VARCHAR(64) NOT NULL DEFAULT 'new user',
    avatar_url      VARCHAR(512),
    native_language native_language_code NOT NULL,
    gems_balance    INT DEFAULT 0,

    active_language language_code NOT NULL,

    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at      TIMESTAMPTZ
);

-- Register / Password Reset Tokens
CREATE TABLE auth_tokens (
    id          UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id     INT NOT NULL REFERENCES users(id),

    token_hash  TEXT NOT NULL,
    purpose     token_purpose NOT NULL,
    used_at     TIMESTAMPTZ,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Login Sessions
CREATE TABLE auth_sessions (
    -- id 将作为 Cookie 的值发给用户
    id         UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id    INT NOT NULL REFERENCES users(id),

    ip_address TEXT,
    user_agent TEXT,

    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX idx_tokens_expires_at ON auth_tokens(expires_at);
CREATE INDEX idx_sessions_expires_at ON auth_sessions(expires_at);
```

```postgresql
-- Profiles for each language the user is learning
CREATE TABLE user_learning_profiles (
    user_id           INT NOT NULL REFERENCES users(id),
    language          language_code NOT NULL,
    level_self_assign INT NOT NULL DEFAULT 2 CHECK (level_self_assign BETWEEN 1 AND 5),

    -- 用于记录该语言下的独立进度（如该语言下的总积分、streak 等）
    -- 以后可以在此扩展该语言的特定设置

    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (user_id, language)
);
```

#### 1.1.2 `tasks` — 任务池

一个任务在沿用框架的前提下设计出多种变体，如评论不同AO3小说、拒绝不同种类的邀请等，因此采取「模板+槽位」设计。

任务和背景材料示例：[Google AI Studio](https://aistudio.google.com/app/prompts?state=%7B%22ids%22:%5B%221ygsEx7oOwm3euPDWDElIsiQmkoIk8G7b%22%5D,%22action%22:%22open%22,%22userId%22:%22104429399333212434405%22,%22resourceKeys%22:%7B%7D%7D&usp=sharing)

```postgresql
CREATE TYPE task_type AS ENUM (   -- 系统调用LLM的模式
    'chat',          -- 聊天, 发言即时且简短, 涉及至少1个agent
    'oneshot',       -- 用户一次写成, 通常内容很长, 不涉及agent
    'slow',          -- 留言或邮件等, 有较长的回复间隔, 涉及1个agent
    'translate'      -- 翻译, 通常为每日任务, 不涉及agent
);
CREATE TYPE ui_variant AS ENUM (  -- 前端展示的界面, 和context格式紧密相关
    'reddit', 'apple_mail', 'discord', 'imessage', 'ao3', 'translator'
);
CREATE TYPE task_cadence AS ENUM ('weekly', 'daily');

CREATE TABLE tasks (
  id            INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  is_active     BOOLEAN       NOT NULL DEFAULT TRUE,
  language      language_code NOT NULL,
  type          task_type     NOT NULL,
  ui            ui_variant    NOT NULL,
  cadence       task_cadence  NOT NULL,
  UNIQUE (id, language),

  -- 以下内容字段均使用任务本身的语言
  -- 模板用 {{slot_name}} 标记可替换槽位, 槽位内容为纯文本
  -- "Write a positive comment for {{fic_title}} of {{fandom}} on AO3"
  title_template       VARCHAR(200) NOT NULL,
  description_template TEXT,   -- 具体情境信息, HTML格式, 支持槽位
  objectives_template  JSONB,  -- 目标, 支持槽位
  /* objectives 示例:
    [
      {"order": 1, "text": "Give a convincing reason for not showing up"},
      {"order": 2, "text": "Do not over-explain"},
      {"order": 3, "text": "Show you still value their invitation and the friendship"}
    ]
  */
  agent_prompt_template  TEXT,  -- 系统提示词, 支持槽位
  agent_persona_pool     JSONB,
  /* 每次开始 session 时随机抽一个, 拼在 system prompt 的前面
    [
      {
        "name": "Marco",
        "age": 28,
        "personality": "......sarcastic but helpful",
        "background": "Works in Milan tech startup..."
    	}
    ]
  */

  background_html TEXT,     -- 预备材料富文本, 不应受槽位内容变化影响
  candidates      JSONB,
  /* candidates 包含slots和context, 覆盖此任务模板的每一种变体
     context: 针对此任务类型设计的页面结构json, 即上下文或其他用户的已有发言, 若为translator则是要翻译的内容
     示例:
    [
  		{
  			"slots": {
  				"fic_title": "Draco Malfoy and the Mortifying Ordeal of Being in Love",
  				"fandom": "Harry Potter",
  				"url": "https://archiveofourown.org/works/34500952"
  			},
  			"context": { ...例如作品名, summary和几条已有评论...}
  		},
      {...其他版本...}
  	]
  */

  max_turns       INT NOT NULL, -- 交互轮数上限, 0对应oneshot/translate, 无需回应
  estimated_words INT,          -- 预估文字量
  difficulty      INT CHECK (difficulty BETWEEN 1 AND 5),
  point_reward    INT,
  gem_reward      INT,

  -- 自动排期调度字段, 每次被排入 schedules 表时更新为当时时间
  last_scheduled_at   TIMESTAMPTZ NOT NULL DEFAULT now(),

  created_by          INT REFERENCES users(id),
  created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

设定点数、宝石默认奖励的触发器：

```postgresql
CREATE OR REPLACE FUNCTION fn_set_task_rewards()
RETURNS TRIGGER AS $$
BEGIN
    -- 1. 处理 point_reward 的默认值
    IF NEW.point_reward IS NULL THEN
        IF NEW.cadence = 'weekly' THEN
            NEW.point_reward := 3;
        ELSIF NEW.cadence = 'daily' THEN
            NEW.point_reward := 1;
        END IF;
    END IF;

    -- 2. 处理 gem_reward 的默认值: point_reward 翻 10 倍
    IF NEW.gem_reward IS NULL THEN
        NEW.gem_reward := COALESCE(NEW.point_reward, 0) * 10;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_set_rewards
BEFORE INSERT ON tasks
FOR EACH ROW
EXECUTE FUNCTION fn_set_task_rewards();
```

通用更新触发器：

```postgresql
CREATE OR REPLACE FUNCTION fn_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at := now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_users_updated_at
  BEFORE UPDATE ON users FOR EACH ROW EXECUTE FUNCTION fn_set_updated_at();

CREATE TRIGGER trg_tasks_updated_at
  BEFORE UPDATE ON tasks FOR EACH ROW EXECUTE FUNCTION fn_set_updated_at();
```

#### 1.1.3 `task_schedules` — 每周/每日排期

```postgresql
CREATE TYPE schedule_origin AS ENUM (
    'manual',    -- 管理员手动指定
    'auto'       -- 系统自动填充
);

CREATE TABLE task_schedules (
    id        INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  	task_id   INT NOT NULL,
    language  language_code NOT NULL,
    CONSTRAINT fk_task_identity
      FOREIGN KEY (task_id, language)
      REFERENCES tasks(id, language),

    date      DATE NOT NULL, -- 周任务存当周周一, 日任务存当天日期
    origin    schedule_origin NOT NULL,

    -- 排期时从对应任务的各槽位候选值中随机抽取后, 将最终值快照存于此
    -- 这样同一任务在不同排期时呈现的具体内容可以不同
    -- 且前端无需自行组装, 直接读取即可 (但仍要组装persona)
    resolved_title        VARCHAR(200) NOT NULL,
    resolved_description  TEXT,
    resolved_objectives   JSONB,
    resolved_agent_prompt TEXT,
    resolved_context      JSONB,

    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (date, task_id)
);

CREATE INDEX idx_schedules_lang_date ON task_schedules(language, date);
```

**注**：因为 task 是未组装的，任务大厅向用户提供的应当是 `schedule_id` 而不是 `task_id`

#### 1.1.4 `practice_sessions` — 用户任务会话

```postgresql
CREATE TYPE session_status AS ENUM ('in_progress', 'completed', 'evaluated');

CREATE TABLE practice_sessions (
  id            INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  user_id       INT NOT NULL REFERENCES users(id),
  schedule_id   INT NOT NULL REFERENCES task_schedules(id),

  agent_prompt_snapshot JSONB, -- resolved_agent_prompt+抽到的persona
  status          session_status NOT NULL DEFAULT 'in_progress',
  tutor_feedback  JSONB,

  started_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  completed_at    TIMESTAMPTZ
);

CREATE INDEX idx_sessions_user_schedule ON practice_sessions(user_id, schedule_id);
```

问题：是否需要限制同一用户对同一 schedule 重复开始会话？

#### 1.1.5 `session_messages` — 对话消息

```postgresql
CREATE TYPE message_role AS ENUM ('user', 'agent', 'tutor', 'hint');

CREATE TABLE session_messages (
  id           BIGINT  GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  session_id   INT          NOT NULL REFERENCES practice_sessions(id),
  role         message_role NOT NULL,
  content      TEXT         NOT NULL,
  llm_metadata JSONB,  -- token数, 耗时, 模型信息等
  created_at   TIMESTAMPTZ  NOT NULL DEFAULT now()
);

CREATE INDEX idx_messages_session ON session_messages(session_id);
```

### 1.2 beta 阶段

#### 1.2.1 `review_cards` — 复习卡片（缓存）

#### 1.2.2 `review_logs` — 复习记录

#### 1.2.3 周连胜 streak

可能可以直接alter user_learning_profiles

#### 1.2.4 `notebook_entries` — 个人笔记本

- 用户既可以在tutor的反馈上标记笔记，也可以在背景材料中选择内容加入笔记本。前端上这可能需要通过特定的hook实现，加入笔记本时既需要包括选中词，又需要携带其所在的句子/语境，以便AI生成相关复习材料。
- 直接在笔记条目上存储 FSRS 参数，避免多表 JOIN。

### 1.3 gamma 阶段

#### 1.3.1 `tags` & `task_tags` — 标签系统

#### 1.3.2 `admin_feedback_inbox` — 学习者反馈收集

#### 1.3.3 `gem_transactions` 表

---

## 2. RESTful API Docs

**Base URL:** `https://api.libiamo.uk/v1`

**Content-Type:** `application/json`

**认证方式:** Cookie-based。登录成功后服务端通过 `Set-Cookie: libiamo_session=<session_uuid>; HttpOnly; Secure; SameSite=Lax; Path=/` 下发凭证，后续请求浏览器自动携带。

**分页格式：**`?page=1&page_size=20`，响应包含 `total_count` / `page` / `page_size`
**时间格式：**ISO-8601 UTC（`2025-07-13T08:00:00Z`），每周的开始为周一

**成功响应:** 直接返回所需 JSON 数据，HTTP 2xx。

**失败响应:** HTTP 4xx/5xx，统一格式：

```json
{
  "err": "ERROR_CODE",
  "message": "Human-readable explanation"
}
```

**通用错误码：**

| HTTP | err | 说明 |
|---|---|---|
| 401 | `AUTH_REQUIRED` | 未携带有效 session cookie |
| 403 | `FORBIDDEN` | 角色权限不足 |
| 404 | `NOT_FOUND` | 资源不存在 |
| 422 | `VALIDATION_ERROR` | 请求体校验失败，`message` 中包含字段级细节 |
| 429 | / | 请求频率超限（由反向代理处理，不会返回JSON） |

以下各节仅列出**端点特有**的错误码，通用错误码不再重复。

**约定：** 🆓 = 无需认证 · 🔒 = 需 Learner 或 Admin · 👑 = 仅 Admin

---

### 2.1 Authentication

#### 2.1.1 注册

`POST /auth/register` 🆓

**Request Body**

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `email` | string | ✅ | 最长 128 字符 |
| `password` | string | ✅ | 8–72 字符，至少含大小写字母和数字各一 |
| `target_languages` | array[string] | ✅ | 允许包含 `"en"` / `"es"` / `"fr"` |
| `native_language` | string | ✅ | BCP 47 格式，如 `"zh"` `"zh-CN"` `"ja"`，由前端自动附上 |
| `timezone` | string | | IANA 时区，默认值 `"UTC"`，由前端自动附上 |

**201 Created**

```json
{
  "email": "learner@example.com",
  "target_languages": ["en"],
  "native_language": "zh-CN",
  "created_at": "2025-07-14T08:00:00Z"
}
```

| HTTP | err | 触发条件 |
|---|---|---|
| 409 | `EMAIL_ALREADY_EXISTS` | 邮箱已注册且对应用户已验证 |
| 429 | `TOO_MANY_REQUESTS` | 未验证用户20分钟内再次以相同邮箱注册 |

> **Note:** 注册后发送验证邮件。邮件中链接携带明文 token 作为 query 参数，服务端存储 token 的 SHA-256 哈希。未验证的用户(1)无法登录，(2)请求重置密码不会执行，(3)可以在20分钟后覆盖注册，20分钟内尝试再注册时系统返回 429 Too Many Requests。

#### 2.1.2 验证邮箱

`POST /auth/verify-email` 🆓

这个端点既用于注册，也用于修改密码时的邮箱验证。若验证成功，用户获得Cookie，新注册用户会被前端重定向至主页，试图重置密码的用户则会被前端重定向至修改密码页。

**Request Body**

| 字段 | 类型 | 必填 |
|---|---|---|
| `token` | string | ✅ |

**204 No Content** — 成功，无响应体，下发 `Set-Cookie` 头。

| HTTP | err | 触发条件 |
|---|---|---|
| 400 | `TOKEN_INVALID` | token 不存在或已使用 |
| 410 | `TOKEN_EXPIRED` | token 已过期 |

#### 2.1.3 登录

`POST /auth/login` 🆓

**Request Body**

| 字段 | 类型 | 必填 |
|---|---|---|
| `email` | string | ✅ |
| `password` | string | ✅ |

**200 OK** — 下发 `Set-Cookie` 头；返回与 `GET /users/me` 相同的内容。

| HTTP | err | 触发条件 |
|---|---|---|
| 401 | `INVALID_CREDENTIALS` | 邮箱或密码错误 |
| 403 | `EMAIL_NOT_VERIFIED` | 账号尚未完成邮箱验证 |

#### 2.1.4 登出

`POST /auth/logout` 🔒

**204 No Content** — 服务端删除该 session 行，并下发 `Set-Cookie` 清除 cookie。

#### 2.1.5 未登录状态下请求重置密码

`POST /auth/password-reset/request` 🆓

**Request Body**

| 字段 | 类型 | 必填 |
|---|---|---|
| `email` | string | ✅ |

**202 Accepted** — 无论邮箱是否存在均返回 202，防止枚举。

```json
{
  "message": "If an account with this email exists, a reset link has been sent."
}
```

#### 2.1.6 指定新密码

`POST /auth/password-reset/confirm` 🔒

**Request Body**

| 字段 | 类型 | 必填 |
|---|---|---|
| `new_password` | string | ✅ |

**204 No Content**

| HTTP | err | 触发条件 |
|---|---|---|
| 400 | `TOKEN_INVALID` | token 无效或已使用 |
| 410 | `TOKEN_EXPIRED` | token 已过期 |

---

### 2.2 User Profile & Progress

#### 2.2.1 获取当前用户信息

`GET /users/me` 🔒

**200 OK**

```json
{
  "id": 1,
  "email": "learner@example.com",
  "role": "learner",
  "nickname": "Aria",
  "avatar_url": "https://cravatar.cn/avatar/[邮箱MD5值]",
  "native_language": "zh-CN",
  "timezone": "Asia/Shanghai",
  "gems_balance": 120,
  "active_language": "en",
  "languages": [
    { "code": "en", "level_self_assign": 3 },
    { "code": "es", "level_self_assign": 1 }
  ],
  "created_at": "2025-07-01T10:00:00Z",
  "updated_at": "2025-07-14T08:30:00Z"
}
```

#### 2.2.2 更新个人信息/切换学习语言【BETA阶段再做】

`PATCH /users/me` 🔒

**Request Body** — 仅传需要修改的字段

**200 OK** — 返回更新后的完整用户对象（同 2.2.1）。

#### 2.2.3 我的所有练习记录

`GET /users/me/sessions` 🔒

**Query Parameters**

| 参数 | 类型 | 默认 | 说明 |
|---|---|---|---|
| `status` | string | — | 可选过滤：`in_progress` / `completed` / `evaluated` |
| `page` | int | 1 | |
| `per_page` | int | 20 | 最大 50 |

**200 OK**

```json
{
  "data": [
    {
      "session_id": 42,
      "schedule_id": 101,
      "title": "Decline a dinner invitation from a friend",
      "type": "chat",
      "ui": "imessage",
      "cadence": "weekly",
      "status": "evaluated",
      "point_reward": 3,
      "gem_reward": 30,
      "started_at": "2025-07-14T09:00:00Z",
      "completed_at": "2025-07-14T09:25:00Z"
    }
  ],
  "pagination": {
    "page": 1,
    "per_page": 20,
    "total": 8
  }
}
```

---

### 2.3 Admin 任务管理

以下的 `id` 默认均指 task_id。

#### 2.3.1 创建任务

`POST /tasks` 👑

**Request Body**

```json
{
  "language": "en",
  "type": "chat",
  "ui": "imessage",
  "cadence": "weekly",

  "title_template": "Decline {{event_type}} invitation from {{relation}}",
  "description_template": "<p>Your {{relation}} has invited you to {{event_type}}...</p>",
  "objectives_template": [
    { "order": 1, "text": "Give a convincing reason" },
    { "order": 2, "text": "Do not over-explain" },
    { "order": 3, "text": "Show you still value the friendship" }
  ],
  "agent_prompt_template": "The user is your {{relation}}. You just invited them to {{event_type}}. Stay in character...",
  "agent_persona_pool": [
    {
      "name": "Marco",
      "age": 28,
      "personality": "sarcastic but caring",
      "background": "Works at a Milan tech startup"
    },
    {
      "name": "Sophie",
      "age": 34,
      "personality": "warm and understanding",
      "background": "Elementary school teacher in Lyon"
    }
  ],
  "background_html": "<h2>How to politely decline in English</h2><p>...</p>",
  "candidates": [
    {
      "slots": {
        "event_type": "a dinner party",
        "relation": "college friend"
      },
      "context": {
        "previous_messages": [
          { "sender": "agent", "text": "Hey! I'm throwing a dinner party this Saturday..." }
        ]
      }
    },
    {
      "slots": {
        "event_type": "a weekend hiking trip",
        "relation": "coworker"
      },
      "context": {
        "previous_messages": [
          { "sender": "agent", "text": "A few of us are heading to the mountains this weekend..." }
        ]
      }
    }
  ],
  "max_turns": 10,
  "estimated_words": 150,
  "difficulty": 3,
  "point_reward": null,
  "gem_reward": null
}
```

> `point_reward` / `gem_reward` 传 `null` 或省略时触发数据库默认值触发器。

**201 Created** — 返回完整的 task 对象（含服务端生成的 `id`、`created_at` 等）。

#### 2.3.2 任务池

`GET /tasks` 👑

**Query Parameters**

| 参数 | 类型 | 默认 | 说明 |
|---|---|---|---|
| `language` | string | — | 筛选语言 |
| `cadence` | string | — | `weekly` / `daily` |
| `type` | string | — | `chat` / `oneshot` / `slow` / `translate` |
| `is_active` | bool | — | |
| `page` | int | 1 | |
| `per_page` | int | 20 | 最大 100 |

**200 OK**

```json
{
  "data": [
    {
      "id": 5,
      "language": "en",
      "type": "chat",
      "ui": "imessage",
      "cadence": "weekly",
      "title_template": "Decline {{event_type}} invitation from {{relation}}",
      "difficulty": 3,
      "max_turns": 10,
      "estimated_words": 150,
      "point_reward": 3,
      "gem_reward": 30,
      "is_active": true,
      "candidates_count": 2,
      "last_scheduled_at": null,
      "created_at": "2025-07-10T06:00:00Z"
    }
  ],
  "pagination": {
    "page": 1,
    "per_page": 20,
    "total": 12
  }
}
```

#### 2.3.3 任务具体信息

`GET /tasks/<id>` 👑

**200 OK** — 返回完整 task 对象，包含所有模板字段、`candidates`、`agent_persona_pool`、`background_html` 等。

#### 2.3.4 更新任务

`PATCH /tasks/<id>` 👑

**Request Body** — 仅传需要修改的字段，字段名与创建时一致。

**200 OK** — 返回更新后的完整 task 对象。

| HTTP | err | 触发条件 |
|---|---|---|
| 409 | `TASK_HAS_ACTIVE_SESSIONS` | 修改关键字段(`type|ui|max_turns|language|cadence`) 时存在进行中的 session |

> **Note:** 将 `is_active` 设为 `false` 即为软删除。已产生的排期和历史 session 不受影响。

---

### 2.4 Schedules（任务大厅）

以下的 `id` 默认均指 schedule_id。

#### 2.4.1 获取任务列表（用于主页渲染）

`GET /schedules` 🔒

基于当前用户的 `active_language` 自动筛选。

**Query Parameters**

| 参数 | 类型 | 说明 |
|---|---|---|
| `date` | string (date) | 默认为当天，按用户时区；若传入则返回该日期所在周的数据 |

**200 OK**

```json
{
  "week_start": "2026-03-16",
  "points_earned": 4,
  "points_required": 6,
  "is_lit": false,
  "weekly": [
    {
      "id": 101,
      "title": "Decline a dinner party invitation from a college friend",
      "cadence": "weekly",
      "type": "chat",
      "ui": "imessage",
      "difficulty": 3,
      "estimated_words": 150,
      "point_reward": 3,
      "gem_reward": 30,
      "date": "2026-03-16",
      "my_session": null
    }, {...}, {...}
  ],
  "daily": [
    {
      "id": 102,
      "title": "Write a positive comment for 'DMHOLE' on AO3",
      "cadence": "weekly",
      "type": "oneshot",
      "ui": "ao3",
      "difficulty": 2,
      "estimated_words": 200,
      "point_reward": 1,
      "gem_reward": 10,
      "date": "2026-03-17",
      "my_session": {
        "id": 42,
        "status": "in_progress"
      }
    }, {...}, {...}
  ]
}
```

> **`my_session`：** 当前用户在该排期下是否已有会话。`null` = 未开始；有值则包含 `id` 与 `status`。前端据此显示「开始」/「继续」/「查看反馈」。
>
> **`is_lit`：** `points_earned >= points_required` 时为 `true`，代表本周已"点亮"。
>
> **此结构在后续加入 streak 功能时需要调整。****

#### 2.4.2 任务详情

`GET /schedules/<id>` 🔒

**200 OK**

```json
{
  "id": 101,
  "date": "2025-07-14",
  "cadence": "weekly",
  "type": "chat",
  "ui": "imessage",
  "difficulty": 3,
  "estimated_words": 150,
  "max_turns": 10,
  "point_reward": 3,
  "gem_reward": 30,
  "origin": "auto",

  "title": "Decline a dinner party invitation from a college friend",
  "description": "<p>Your college friend has invited you to a dinner party this Saturday. You already have plans...</p>",
  "objectives": [
    { "order": 1, "text": "Give a convincing reason for not showing up" },
    { "order": 2, "text": "Do not over-explain" },
    { "order": 3, "text": "Show you still value the friendship" }
  ],
  "context": {
    "previous_messages": [
      {
        "sender": "agent",
        "text": "Hey! I'm throwing a dinner party this Saturday, would love for you to come! 🎉"
      }
    ]
  },
  "background_html": "<h2>Declining invitations in English</h2><p>When turning down an invitation, native speakers typically...</p>",

  "my_session": null
}
```

> **Note:** `resolved_agent_prompt` 不暴露给前端。`background_html` 来源于 `tasks.background_html`（通过 JOIN 获取）。

#### 2.4.3 创建排期

`POST /schedules` 👑

**Request Body**

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `task_id` | int | ✅ | 目标任务 |
| `date` | string (date) | ✅ | 周任务为周一日期，日任务为当天 |
| `candidate_index` | int | | 指定使用 `candidates` 数组中的第几项（0-based）。省略则随机 |

**201 Created**

```json
{
  "id": 115,
  "task_id": 5,
  "language": "en",
  "date": "2025-07-21",
  "origin": "manual",
  "title": "Decline a weekend hiking trip invitation from a coworker",
  "created_at": "2025-07-14T12:00:00Z"
}
```

| HTTP | err | 触发条件 |
|---|---|---|
| 409 | `SCHEDULE_CONFLICT` | 同一 `date` + `task_id` 已存在排期 |
| 422 | `TASK_INACTIVE` | 目标任务 `is_active = false` |
| 422 | `CANDIDATE_INDEX_OUT_OF_RANGE` | 指定的索引超出 `candidates` 数组范围 |

> **Side Effect:** 更新 `tasks.last_scheduled_at` 为当前时间。

#### 2.4.4 删除排期

`DELETE /schedules/<id>` 👑

**204 No Content**

| HTTP | err | 触发条件 |
|---|---|---|
| 409 | `SCHEDULE_HAS_SESSIONS` | 已有用户基于此排期开始了练习会话 |

---

### 2.5 Practice Sessions（练习会话）

以下的 `id` 默认均指 session_id。

#### 2.5.1 开始练习

`POST /sessions` 🔒

**Request Body**

| 字段 | 类型 | 必填 |
|---|---|---|
| `schedule_id` | int | ✅ |

**201 Created**

```json
{
  "id": 42,
  "schedule_id": 101,
  "status": "in_progress",

  "type": "chat",
  "ui": "imessage",
  "max_turns": 10,

  "title": "Decline a dinner party invitation from a college friend",
  "objectives": [
    { "order": 1, "text": "Give a convincing reason for not showing up" },
    { "order": 2, "text": "Do not over-explain" },
    { "order": 3, "text": "Show you still value the friendship" }
  ],
  "context": {
    "previous_messages": [
      { "sender": "agent", "text": "Hey! I'm throwing a dinner party this Saturday, would love for you to come! 🎉" }
    ]
  },

  "agent": {
    "name": "Marco",
    "age": 28
  },

  "messages": [],
  "started_at": "2025-07-14T09:00:00Z"
}
```

| HTTP | err | 触发条件 |
|---|---|---|
| 409 | `SESSION_ALREADY_EXISTS` | 该用户在此排期下已有会话 |

> **后端行为：** 从 `agent_persona_pool` 中随机抽取一个 persona，拼接 `resolved_agent_prompt` 后存入 `agent_prompt_snapshot`。仅将 persona 的非敏感字段（`name`、`age`）返回前端。

#### 2.5.2 获取会话详情

`GET /sessions/<sessionId>` 🔒

用于页面刷新后恢复会话，或查看已完成会话的反馈。返回渲染会话 UI 所需的全部信息。

**200 OK**

```json
{
  ...此部分同 POST /sessions...,

  "messages": [
    {
      "id": 1,
      "role": "user",
      "content": "Hey Marco, thanks for the invite! I'd love to but I already have plans...",
      "created_at": "2025-07-14T09:01:00Z"
    },
    {
      "id": 2,
      "role": "agent",
      "content": "Oh no! What kind of plans? Can't you reschedule?",
      "created_at": "2025-07-14T09:01:03Z"
    },
    {
      "id": 3,
      "role": "hint",
      "content": "Consider using phrases like 'I'm afraid I can't make it' or 'Rain check?'",
      "created_at": "2025-07-14T09:05:00Z"
    },
    {
      "id": 4,
      "role": "user",
      "content": "I'm afraid it's a family thing, so I really can't move it. Rain check though?",
      "created_at": "2025-07-14T09:06:00Z"
    },
    {
      "id": 5,
      "role": "agent",
      "content": "No worries at all! Family first. Let's definitely do something next week then 😊",
      "created_at": "2025-07-14T09:06:04Z"
    }
  ],

  "tutor_feedback": {
    "content": "Good job overall! You handled the refusal tactfully... You said 'I'd love to but I already have plans.' This is a solid hedge. You could intensify the positive with 'That sounds amazing, but...'...",
    "objective_results": [
      { "order": 1, "text": "Give a convincing reason for not showing up", "met": true },
      { "order": 2, "text": "Do not over-explain", "met": true },
      { "order": 3, "text": "Show you still value the friendship", "met": true }
    ]
  },

  "started_at": "2025-07-14T09:00:00Z",
  "completed_at": "2025-07-14T09:10:00Z"
}
```

> **权限：** 用户只能访问自己的 session。管理员可访问任意 session。

#### 2.5.3 发送消息

`POST /sessions/<sessionId>/messages` 🔒

用户发送一条消息。对于chat和slow类型任务，响应中包含Agent的回复（前端可以先存下来，但可能需要延迟显示，比如chat任务延迟2秒播放一个对方正在输入的提示，slow任务开始一个1分钟的计时，直到时间到才显示）；对于oneshot和translate类型任务，不返回Agent回复。

**Request Body**

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `content` | string | ✅ | 用户消息文本，最长 10000 字符 |

**200 OK** — `chat` / `slow` 类型：

```json
{
  "user_message": {
    "id": 4,
    "role": "user",
    "content": "I'm afraid it's a family thing, so I really can't move it. Rain check though?",
    "created_at": "2025-07-14T09:06:00Z"
  },
  "agent_message": {
    "id": 5,
    "role": "agent",
    "content": "No worries at all! Family first. Let's definitely do something next week then 😊",
    "llm_metadata": {
      "model": "gpt-4o-mini",
      "tokens_used": 187,
      "latency_ms": 1420
    },
    "created_at": "2025-07-14T09:06:04Z"
  },
  "turn_count": 3,
  "turns_remaining": 7
}
```

**200 OK** — `oneshot` / `translate` 类型：

```json
{
  "user_message": {
    "id": 10,
    "role": "user",
    "content": "Dear Professor Chen, I am writing to request...",
    "created_at": "2025-07-14T09:15:00Z"
  },
  "agent_message": null,
  "turn_count": 1,
  "turns_remaining": 0
}
```

| HTTP | err | 触发条件 |
|---|---|---|
| 409 | `SESSION_ALREADY_COMPLETED` | 会话已标记完成 |
| 409 | `MAX_TURNS_REACHED` | 已达到 `max_turns` 上限 |

> **`turn_count` / `turns_remaining`：** 一"轮"定义为一次用户发言（不论是否有 Agent 回复）。`turns_remaining = max_turns - turn_count`。

#### 2.5.4 请求提示

`POST /sessions/<sessionId>/hints` 🔒

当用户写作中因词汇或思路卡顿时，向 Tutor Agent 请求帮助。提示消息以 `role = "hint"` 存入消息记录，**不消耗交互轮数**。

**Request Body**

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `stuck_on` | string | ✅ | 用户遇到的困难描述，如 "I don't know how to politely insist" |
| `draft` | string | | 用户当前正在编辑的草稿（可选，帮助 AI 给出更精准的提示） |

**200 OK**

```json
{
  "hint": {
    "id": 3,
    "role": "hint",
    "content": "To politely insist while declining, you could try:\n• 'I really wish I could, but...'\n• 'I appreciate you thinking of me, however...'\n\nThese phrases show warmth while maintaining your refusal.",
    "created_at": "2025-07-14T09:05:00Z"
  }
}
```

| HTTP | err | 触发条件 |
|---|---|---|
| 409 | `SESSION_ALREADY_COMPLETED` | 会话已完成 |

#### 2.5.5 完成会话并获取反馈

`POST /sessions/<sessionId>/complete` 🔒

用户标记任务完成，系统将对话记录打包发送给评审 Agent 进行评估，返回反馈和奖励信息。

**Request Body** — 无。

**200 OK** 无返回内容

| HTTP | err | 触发条件 |
|---|---|---|
| 409 | `SESSION_ALREADY_COMPLETED` | 会话已完成或已评估 |
| 422 | `SESSION_NO_MESSAGES` | 用户未发送任何消息即尝试完成 |

> **后端流程：**
> 1. 将 session 状态更新为 `completed`
> 2. 打包所有消息，发送给评审 Agent
> 3. 存储 `tutor_feedback` 到 `practice_sessions.tutor_feedback`
> 4. 计算并发放奖励（更新 `users.gems_balance`）
> 5. 更新状态为 `evaluated`，写入 `completed_at`
>
>若评估 LLM 调用失败，状态停留在 `completed`，隔一段（越来越长的）时间重试。

---

### 2.6 alpha 阶段端点速查表

| Method | Path | Auth | 说明 |
|---|---|---|---|
| `POST` | `/auth/register` | 🆓 | 注册 |
| `POST` | `/auth/verify-email` | 🆓 | 邮箱验证 |
| `POST` | `/auth/login` | 🆓 | 登录 |
| `POST` | `/auth/logout` | 🔒 | 登出 |
| `POST` | `/auth/password-reset/request` | 🆓 | 申请密码重置 |
| `POST` | `/auth/password-reset/confirm` | 🔒 | 指定新密码 |
| `GET` | `/users/me` | 🔒 | 获取个人信息 |
| `PATCH` | `/users/me` | 🔒 | 修改个人信息 |
| `GET` | `/users/me/sessions` | 🔒 | 练习历史 |
| `POST` | `/tasks` | 👑 | 创建任务 |
| `GET` | `/tasks` | 👑 | 任务列表 |
| `GET` | `/tasks/<taskId>` | 👑 | 任务详情 |
| `PATCH` | `/tasks/<taskId>` | 👑 | 更新任务 |
| `GET` | `/schedules` | 🔒 | 任务大厅（排期列表） |
| `GET` | `/schedules/<scheduleId>` | 🔒 | 排期详情（含背景材料） |
| `POST` | `/schedules` | 👑 | 创建排期 |
| `DELETE` | `/schedules/<scheduleId>` | 👑 | 删除排期 |
| `POST` | `/sessions` | 🔒 | 开始练习会话 |
| `GET` | `/sessions/<sessionId>` | 🔒 | 会话详情（含消息与反馈） |
| `POST` | `/sessions/<sessionId>/messages` | 🔒 | 发送消息 |
| `POST` | `/sessions/<sessionId>/hints` | 🔒 | 请求写作提示 |
| `POST` | `/sessions/<sessionId>/complete` | 🔒 | 完成会话 |
