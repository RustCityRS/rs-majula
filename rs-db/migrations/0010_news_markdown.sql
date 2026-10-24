-- The Markdown a post was written in, from the staff console. `content` stays
-- what the website shows: the HTML rendered from it when the post is saved.
-- NULL for posts written as HTML.
ALTER TABLE news_posts ADD COLUMN markdown TEXT;
