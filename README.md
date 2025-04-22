# Oxygen

## Giới thiệu

Oxygen là dịch vụ tài chính phi tập trung (DeFi prime brokerage service) được xây dựng trên nền tảng Solana và được hỗ trợ bởi cơ sở hạ tầng on-chain của Serum. Được thiết kế để hỗ trợ hàng trăm triệu người dùng, Oxygen là một giao thức phi tập trung, chi phí thấp và có khả năng mở rộng cao, giúp dân chủ hóa việc vay mượn, cho vay và giao dịch với đòn bẩy, cho phép người dùng tận dụng tối đa nguồn vốn của họ.

## Tính năng chính

Với Oxygen, bạn có thể:

- **Tạo lợi nhuận (yield)** từ các tài sản của mình
- **Vay từ những người dùng khác** dựa trên tài sản thế chấp
- **Giao dịch trực tiếp** từ các nhóm tài sản của bạn
- **Sử dụng đòn bẩy giao dịch** dựa trên danh mục tài sản đa dạng

## Điểm khác biệt

Oxygen khác biệt với các giao thức cho vay/đi vay khác ở ba điểm chính:

### 1. Sử dụng cùng một tài sản thế chấp cho nhiều mục đích
Giao thức cho phép bạn tạo lợi nhuận từ danh mục đầu tư của mình thông qua việc cho vay tài sản và đồng thời vay các tài sản khác.

### 2. Thế chấp đa dạng (Cross-collateralization)
Bạn có thể sử dụng toàn bộ danh mục đầu tư của mình làm tài sản thế chấp khi muốn vay các tài sản khác, giúp giảm rủi ro margin call và thanh lý cho danh mục đầu tư của bạn.

### 3. Định giá dựa trên thị trường
Giao thức Oxygen dựa trên sổ lệnh (order-book) thay vì tuân theo mô hình thị trường được thiết lập sẵn cần điều chỉnh thủ công.

### 4. Hoàn toàn phi tập trung
- 100% phi tập trung
- 100% phi giám sát
- 100% trên blockchain

Tất cả giao dịch đều là peer-to-peer mà không có sự can thiệp của bất kỳ đơn vị tập trung nào. Giao thức Oxygen không bao giờ có quyền truy cập vào khóa riêng của bạn.

## Kiến trúc dự án

Oxygen được tổ chức theo cấu trúc sau:

```
oxygen-protocol/
├── programs/                         # Chương trình Solana on-chain
│   └── oxygen/                       # Chương trình giao thức chính
├── app/                              # Ứng dụng frontend
├── sdk/                              # SDK TypeScript cho giao thức
├── tests/                            # Kiểm thử tích hợp
└── scripts/                          # Scripts triển khai và tiện ích
```

## Các mô-đun cốt lõi

### 1. Mô-đun quản lý Pool
- Quản lý các nhóm thanh khoản cho các tài sản khác nhau
- Theo dõi tổng số tiền gửi và tiền vay
- Tính toán tỷ lệ sử dụng
- Cập nhật lãi suất dựa trên tỷ lệ sử dụng

### 2. Mô-đun Cho vay/Đi vay
- Xử lý tiền gửi, rút tiền, vay và hoàn trả
- Quản lý các trạng thái tài khoản

### 3. Mô-đun Quản lý tài sản thế chấp
- Thực hiện logic thế chấp đa dạng (cross-collateralization)
- Tính toán giá trị tài sản thế chấp cho danh mục đầu tư của người dùng
- Theo dõi việc sử dụng tài sản thế chấp
- Xác minh khả năng vay

### 4. Mô-đun Giao dịch
- Tích hợp với Serum DEX để giao dịch on-chain
- Tính toán margin có sẵn cho giao dịch đòn bẩy

### 5. Mô-đun Tạo lợi nhuận
- Quản lý phân phối lợi nhuận cho người cho vay
- Tính toán tiền lãi tích lũy

### 6. Mô-đun Thanh lý
- Giám sát các vị thế và thực hiện thanh lý khi cần thiết
- Tính toán hệ số sức khỏe của vị thế

## Bắt đầu

Để bắt đầu với Oxygen:

1. Cài đặt môi trường phát triển Anchor
2. Khởi tạo dự án
3. Xây dựng và kiểm thử chương trình on-chain
4. Phát triển ứng dụng frontend
5. Tạo SDK TypeScript
6. Viết kiểm thử toàn diện

## Liên hệ

Để biết thêm thông tin hoặc hỗ trợ, vui lòng liên hệ với nhóm phát triển.
