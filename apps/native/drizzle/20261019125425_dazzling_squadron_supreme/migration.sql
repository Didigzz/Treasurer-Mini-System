CREATE TABLE `bill_shares` (
	`id` text PRIMARY KEY,
	`bill_id` text NOT NULL,
	`boarder_id` text NOT NULL,
	`boarder_name_snapshot` text NOT NULL,
	`amount_due_minor` integer NOT NULL,
	`created_at` integer NOT NULL,
	CONSTRAINT `fk_bill_shares_bill_id_bills_id_fk` FOREIGN KEY (`bill_id`) REFERENCES `bills`(`id`) ON DELETE CASCADE,
	CONSTRAINT `fk_bill_shares_boarder_id_boarders_id_fk` FOREIGN KEY (`boarder_id`) REFERENCES `boarders`(`id`) ON DELETE RESTRICT
);
--> statement-breakpoint
CREATE TABLE `bill_types` (
	`id` text PRIMARY KEY,
	`name` text NOT NULL,
	`is_archived` integer DEFAULT false NOT NULL,
	`sort_order` integer DEFAULT 0 NOT NULL,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL
);
--> statement-breakpoint
CREATE TABLE `bills` (
	`id` text PRIMARY KEY,
	`bill_type_id` text NOT NULL,
	`bill_type_name_snapshot` text NOT NULL,
	`period_year` integer NOT NULL,
	`period_month` integer NOT NULL,
	`total_minor` integer NOT NULL,
	`due_date` text,
	`note` text,
	`archived_at` integer,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	CONSTRAINT `fk_bills_bill_type_id_bill_types_id_fk` FOREIGN KEY (`bill_type_id`) REFERENCES `bill_types`(`id`) ON DELETE RESTRICT
);
--> statement-breakpoint
CREATE TABLE `boarders` (
	`id` text PRIMARY KEY,
	`name` text NOT NULL,
	`status` text DEFAULT 'active' NOT NULL,
	`archived_at` integer,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL
);
--> statement-breakpoint
CREATE TABLE `payments` (
	`id` text PRIMARY KEY,
	`bill_share_id` text NOT NULL,
	`amount_minor` integer NOT NULL,
	`paid_at` integer NOT NULL,
	`note` text,
	`change_minor` integer DEFAULT 0 NOT NULL,
	`change_returned_at` integer,
	`change_return_note` text,
	`voided_at` integer,
	`void_reason` text,
	`created_at` integer NOT NULL,
	CONSTRAINT `fk_payments_bill_share_id_bill_shares_id_fk` FOREIGN KEY (`bill_share_id`) REFERENCES `bill_shares`(`id`) ON DELETE CASCADE
);
--> statement-breakpoint
CREATE TABLE `settings` (
	`key` text PRIMARY KEY,
	`value` text NOT NULL,
	`updated_at` integer NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX `bill_shares_bill_boarder_idx` ON `bill_shares` (`bill_id`,`boarder_id`);--> statement-breakpoint
CREATE INDEX `bill_shares_boarder_idx` ON `bill_shares` (`boarder_id`);--> statement-breakpoint
CREATE INDEX `bill_types_archived_idx` ON `bill_types` (`is_archived`,`sort_order`);--> statement-breakpoint
CREATE INDEX `bills_period_idx` ON `bills` (`period_year`,`period_month`);--> statement-breakpoint
CREATE INDEX `bills_type_period_idx` ON `bills` (`bill_type_id`,`period_year`,`period_month`);--> statement-breakpoint
CREATE INDEX `boarders_status_idx` ON `boarders` (`status`);--> statement-breakpoint
CREATE INDEX `payments_share_idx` ON `payments` (`bill_share_id`);--> statement-breakpoint
CREATE INDEX `payments_paid_at_idx` ON `payments` (`paid_at`);